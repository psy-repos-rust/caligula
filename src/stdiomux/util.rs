use std::{fmt::Debug, sync::Arc};

use bytes::{Bytes, BytesMut};
use futures::{Stream, StreamExt as _, stream};
use tokio::{
    io::{AsyncRead, AsyncReadExt as _, AsyncWrite, AsyncWriteExt as _, BufReader, BufWriter},
    select,
    sync::SetOnce,
};
use tracing::{Instrument as _, info_span};

pub async fn drive_tx<W>(
    tx: W,
    mut s: impl Stream<Item = Bytes> + Unpin,
) -> Result<(), Arc<std::io::Error>>
where
    W: AsyncWrite + Unpin + 'static,
{
    // wrap with a buffer big enough to wrap the header and a reasonably-sized
    // message
    let mut tx = BufWriter::with_capacity(4096, tx);

    // pull from the request stream
    while let Some(bytes) = s.next().await {
        // don't send 0 bytes because that's a EOF sentinel
        if bytes.is_empty() {
            continue;
        }

        // length-framing
        tx.write_u32(bytes.len().try_into().unwrap()).await?;
        tx.write_all(&bytes).await?;
        tx.flush().await?;
    }

    // out of requests -- write EOF sentinel
    tx.write_u32(0).await?;
    Ok(())
}

pub fn drive_rx<R>(rx: R) -> impl Stream<Item = Result<Bytes, Arc<std::io::Error>>>
where
    R: AsyncRead + Unpin + 'static,
{
    // wrap with a buffer big enough to wrap the header and a reasonably-sized
    // message
    let rx = BufReader::with_capacity(4096, rx);

    stream::unfold(Some(rx), |st| {
        async move {
            let mut rx = st?;

            let recv = async {
                let len = usize::try_from(rx.read_u32().await?).unwrap();
                let mut msg = BytesMut::with_capacity(len);
                unsafe {
                    msg.set_len(len);
                }
                rx.read_exact(&mut msg).await?;
                Ok::<Bytes, Arc<std::io::Error>>(msg.freeze())
            };

            let ret = match recv.await {
                Ok(msg) => (Ok(msg), Some(rx)),
                Err(err) => (Err(err), None),
            };

            Some(ret)
        }
        .instrument(info_span!("rx driver"))
    })
}

pub fn inject_err_stream<S, E>(
    stream: S,
    err_notify: Arc<SetOnce<E>>,
) -> impl Stream<Item = Result<Bytes, E>>
where
    E: Clone,
    S: Stream<Item = Result<Bytes, E>>,
{
    stream
        .map(move |r| match (r, err_notify.get()) {
            (_, Some(err)) => Err(err.clone()), // inject error from err_notify
            (Ok(r), None) => Ok(r),
            (Err(e), None) => {
                // inject error into err_notify
                err_notify.set(e.clone()).ok();
                Err(e)
            }
        })
        .take_while(move |r| std::future::ready(r.is_ok()))
}

pub async fn inject_err_fut<T, E, Fut>(fut: Fut, err_notify: Arc<SetOnce<E>>) -> Result<T, E>
where
    E: Debug + Clone,
    Fut: Future<Output = Result<T, E>>,
{
    let r = select! {
        biased;
        err = err_notify.wait() => { // inject errors from err_notify
            tracing::warn!(?err, "Quitting early due to signalled error");
            Err(err.clone())
        },
        r = fut => r,
    };

    if let Err(err) = &r {
        tracing::warn!(?err, "fut errored, sending signal");
        err_notify.set(err.clone()).ok(); // inject errors into err_notify
    }

    r
}
