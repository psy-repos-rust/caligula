use std::process::Stdio;

use tokio::process::{Child, ChildStdin, ChildStdout};

use crate::{
    escalation::{EscalationError, run_escalate},
    herder_api::{
        self, HerderResponse, HerderService,
        client::HerderClient,
        write_verify::{WVAction, WVError},
    },
};

type Client = HerderClient<ChildStdout, ChildStdin>;

#[derive(Debug, thiserror::Error)]
pub enum ClientTransportError {
    #[error("Daemon management error: {0}")]
    DaemonError(#[from] SpawnDaemonError),
    #[error("Error receiving a message: {0}")]
    Rx(std::io::Error),
    #[error("Error sending a message: {0}")]
    Tx(std::io::Error),
}

impl PartialEq for ClientTransportError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::DaemonError(l0), Self::DaemonError(r0)) => l0 == r0,
            (Self::Rx(_), Self::Rx(_)) => true,
            (Self::Tx(_), Self::Tx(_)) => true,
            _ => false,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SpawnDaemonError {
    #[error("Failed to spawn escalated: {0}")]
    Escalation(#[from] EscalationError),
    #[error("Failed to spawn unescalated: {0}")]
    Standard(std::io::Error),
}

impl PartialEq for SpawnDaemonError {
    fn eq(&self, other: &Self) -> bool {
        matches!(
            (self, other),
            (Self::Escalation(_), Self::Escalation(_)) | (Self::Standard(_), Self::Standard(_))
        )
    }
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
    SpawnDaemonError,
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
            .map_err(SpawnDaemonError::Escalation)?,
        false => {
            let mut c = tokio::process::Command::from(cmd);
            modify_cmd(&mut c);
            c.spawn().map_err(SpawnDaemonError::Standard)?
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

impl HerderService<WVAction> for ChildHerderClient {
    type Error = <Client as HerderService<WVAction>>::Error;

    async fn start(
        &self,
        action: WVAction,
    ) -> Result<Result<HerderResponse<WVAction, Self::Error>, WVError>, Self::Error> {
        self.client.start(action).await
    }
}
