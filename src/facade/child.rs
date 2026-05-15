use std::process::Stdio;

use tokio::process::{Child, ChildStdin, ChildStdout};

use crate::{
    escalation::run_escalate,
    herder_api::{
        self, HerdEvent, HerderResponse, HerderService, client::HerderClient,
        write_verify::WriteVerifyAction,
    },
};

type Client = HerderClient<ChildStdout, ChildStdin>;

#[derive(Debug, thiserror::Error)]
pub enum StartWriterError<E: HerdEvent> {
    #[error("Unexpected first status: {0:?}")]
    UnexpectedFirstStatus(E),
    #[error("Explicit error signaled: {0}")]
    Failed(E::Failure),
    #[error("Daemon management error: {0}")]
    DaemonError(#[from] DaemonError),
    #[error("Communication error: {0}")]
    Comm(std::io::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum DaemonError {
    #[error("Failed to spawn daemon (escalated={0:?}): {1}")]
    DaemonSpawnFailure(bool, anyhow::Error),
}

/// Spawn a child process.
///
/// Returns the [`ChildHerderClient`], along with a driver future that must be
/// polled in the background in order for requests and responses to be handled.
#[tracing::instrument]
pub async fn spawn(
    log_path: String,
    escalated: bool,
) -> Result<
    (
        ChildHerderClient,
        impl Future<Output = Result<(), std::io::Error>>,
    ),
    DaemonError,
> {
    let proc = process_path::get_executable_path().unwrap();
    let cmd = crate::escalation::Command {
        proc: proc.to_str().unwrap().to_owned().into(),
        envs: vec![],
        args: vec!["_herder".into(), log_path.into()],
    };

    tracing::debug!("Starting child process with command: {:?}", cmd);
    fn modify_cmd(cmd: &mut tokio::process::Command) {
        cmd.kill_on_drop(true)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
    }

    let mut child = match escalated {
        true => run_escalate(&cmd, modify_cmd)
            .await
            .map_err(|e| DaemonError::DaemonSpawnFailure(true, e.into()))?,
        false => {
            let mut c = tokio::process::Command::from(cmd);
            modify_cmd(&mut c);
            c.spawn()
                .map_err(|e| DaemonError::DaemonSpawnFailure(false, e.into()))?
        }
    };

    tracing::debug!(?child, "Process spawned");

    let child_rx = child.stdout.take().unwrap();
    let child_tx = child.stdin.take().unwrap();

    let (client, fut) = herder_api::client::create(child_rx, child_tx);

    Ok((
        ChildHerderClient {
            _child: child,
            client,
        },
        fut,
    ))
}

pub struct ChildHerderClient {
    _child: Child,
    client: Client,
}

impl HerderService<WriteVerifyAction> for ChildHerderClient {
    type Error = <Client as HerderService<WriteVerifyAction>>::Error;

    async fn start(
        &self,
        action: WriteVerifyAction,
    ) -> Result<HerderResponse<WriteVerifyAction, Self::Error>, Self::Error> {
        self.client.start(action).await
    }
}
