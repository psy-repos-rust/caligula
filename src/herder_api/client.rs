use futures::{StreamExt, stream};
use tokio::io::{AsyncRead, AsyncWrite};

use crate::{
    facade::StartWriterError,
    herder_api::{
        HerdEvent as _, HerderService, StartHerd, StartWriterResponse, TopLevelHerdEvent,
        write_verify::{WriteVerifyAction, WriteVerifyEvent},
    },
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

impl<R, W> HerderService for HerderClient<R, W>
where
    R: AsyncRead + Unpin + Send + 'static,
    W: AsyncWrite + Unpin + Send + 'static,
{
    type Error = StartWriterError<WriteVerifyEvent>;

    async fn start_writer(
        &self,
        action: WriteVerifyAction,
    ) -> Result<StartWriterResponse<Self::Error>, Self::Error> {
        let (rx, mut tx) =
            self.comm.lock().unwrap().take().expect(
                "called more than once! multiple requests are not currently supported! sowwy!",
            );

        write_msg_async(&mut tx, &StartHerd { id: 0, action })
            .await
            .map_err(StartWriterError::Comm)?;

        // pass tx through or else there will be unexpected EOFs from it closing.
        // TODO: ... fix this thing ...
        let mut stream = Box::pin(stream::unfold((tx, rx), |(tx, mut rx)| async move {
            let msg = read_msg_async::<(u64, TopLevelHerdEvent)>(&mut rx)
                .await
                .map_err(StartWriterError::Comm);
            Some((msg.map(|(_, TopLevelHerdEvent::Writer(msg))| msg), (tx, rx)))
        }));

        let first_msg = stream
            .next()
            .await
            .ok_or(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "didn't get a response from the server",
            ))
            .map_err(StartWriterError::Comm)??;

        let initial_info = first_msg.downcast_as_initial_info().map_err(|other| {
            match other.downcast_as_failure() {
                Ok(error) => StartWriterError::Failed(error),
                Err(other) => StartWriterError::UnexpectedFirstStatus(other),
            }
        })?;

        Ok(StartWriterResponse {
            start: initial_info,
            events: stream,
        })
    }
}
