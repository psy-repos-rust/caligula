use futures::stream;
use tokio::io::{AsyncRead, AsyncWrite};

use crate::{
    facade::ClientTransportError,
    herder_api::{HerderAction, HerderResponse, HerderService, StartHerd},
    ipc_common::{read_msg_async, write_msg_async},
};

/// Create a [`HerderClient`] over the given transport. Returns the client
/// itself, along with a driver future that must be polled in the background in
/// order for requests and responses to be handled.
pub fn create<R, W>(
    rx: R,
    tx: W,
) -> (
    HerderClient<R, W>,
    impl Future<Output = Result<(), std::io::Error>>,
)
where
    R: AsyncRead + Unpin + Send + 'static,
    W: AsyncWrite + Unpin + Send + 'static,
{
    (
        HerderClient {
            comm: Some((rx, tx)).into(),
        },
        std::future::pending(),
    )
}

/// A client to a remote [`HerderService`] over a transport. Created using the
/// [`create()`] function.
///
/// Technically speaking, it only supports one request right now, and explodes
/// afterwards, but that's okay! Refactors will come Soon(tm).
pub struct HerderClient<R, W>
where
    R: AsyncRead + Unpin + Send + 'static,
    W: AsyncWrite + Unpin + Send + 'static,
{
    comm: std::sync::Mutex<Option<(R, W)>>,
}

impl<A, R, W> HerderService<A> for HerderClient<R, W>
where
    A: HerderAction,
    R: AsyncRead + Unpin + Send + 'static,
    W: AsyncWrite + Unpin + Send + 'static,
{
    type Error = ClientTransportError;

    async fn start(
        &self,
        action: A,
    ) -> Result<Result<HerderResponse<A, Self::Error>, A::Error>, Self::Error> {
        // TODO: implement multiplexing

        let (mut rx, mut tx) =
            self.comm.lock().unwrap().take().expect(
                "called more than once! multiple requests are not currently supported! sowwy!",
            );

        write_msg_async(&mut tx, &StartHerd { id: 0, action })
            .await
            .map_err(ClientTransportError::Comm)?;

        let first_msg = read_msg_async::<Result<A::Start, A::Error>>(&mut rx)
            .await
            .map_err(ClientTransportError::Comm)?;

        let start = match first_msg {
            Ok(start) => start,
            Err(err) => return Ok(Err(err)),
        };

        // pass tx through or else there will be unexpected EOFs from it closing.
        // TODO: ... fix this thing ...
        let events = Box::pin(stream::unfold((tx, rx), |(tx, mut rx)| async move {
            let val = read_msg_async::<Result<A::Event, A::Error>>(&mut rx)
                .await
                .map_err(ClientTransportError::Comm);
            Some((val, (tx, rx)))
        }));

        Ok(Ok(HerderResponse { start, events }))
    }
}
