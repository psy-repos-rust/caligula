use std::process::Stdio;

use tokio::process::{Child, ChildStdin, ChildStdout};

use crate::{
    escalation::run_escalate,
    facade::DaemonError,
    herder_api::{
        self, HerderService, StartWriterResponse, client::HerderClient,
        write_verify::WriteVerifyAction,
    },
};

type Client = HerderClient<ChildStdout, ChildStdin>;

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

impl HerderService for ChildHerderClient {
    type Error = <Client as HerderService>::Error;

    async fn start_writer(
        &self,
        action: WriteVerifyAction,
    ) -> Result<StartWriterResponse<Self::Error>, Self::Error> {
        self.client.start_writer(action).await
    }
}
