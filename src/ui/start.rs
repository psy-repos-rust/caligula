use std::{fmt::Display, sync::Arc, time::Instant};

use futures::StreamExt;
use inquire::Confirm;
use tokio::sync::watch;
use tracing::debug;

use crate::{
    device,
    herder_daemon::ipc::{WriteVerifyError, WriteVerifyEvent},
    herder_facade::{HerdHandle, StartWriterError},
    logging::LogPaths,
    orchestrator::{BeginParams, Orchestrator, WriterState},
    ui::{
        cli::{Interactive, UseSudo},
        utils::TUICapture,
    },
};

#[tracing::instrument(skip_all, fields(root, interactive))]
pub async fn try_start_burn(
    orc: &impl Orchestrator,
    args: &BeginParams,
    root: UseSudo,
    interactive: bool,
) -> Result<HerdHandle<WriteVerifyEvent>, StartWriterError<WriteVerifyEvent>> {
    let err = match orc.start_write_verify(false, args.clone()).await {
        Ok(p) => {
            return Ok(p);
        }
        Err(e) => e,
    };

    if let StartWriterError::Failed(WriteVerifyError::PermissionDenied) = &err {
        match (root, interactive) {
            (UseSudo::Ask, true) => {
                debug!("Failure due to insufficient perms, asking user to escalate");

                let response = Confirm::new(&format!(
                    "We don't have permissions on {}. Escalate using sudo?",
                    args.target.name
                ))
                .with_default(true)
                .with_help_message(
                    "We will use the sudo command, which may prompt you for a password.",
                )
                .prompt()
                .expect("prompting the user should not fail");

                if response {
                    return Ok(orc.start_write_verify(true, args.clone()).await?);
                }
            }
            (UseSudo::Always, _) => {
                return Ok(orc.start_write_verify(true, args.clone()).await?);
            }
            _ => {}
        }
    }

    Err(err.into())
}

pub async fn begin_writing(
    interactive: Interactive,
    params: BeginParams,
    handle: HerdHandle<WriteVerifyEvent>,
    log_paths: Arc<LogPaths>,
) -> anyhow::Result<()> {
    debug!("Opening TUI");
    if interactive.is_interactive() {
        debug!("Using fancy interactive TUI");
        let mut tui = TUICapture::new()?;
        let terminal = tui.terminal();

        let (tx, rx) = watch::channel(WriterState::initial(
            Instant::now(),
            !params.compression.is_identity(),
            handle.initial_info.input_file_bytes,
        ));

        let mut events = handle.events;
        let _jh = tokio::spawn(async move {
            while !tx.borrow().is_finished() {
                let event = events.next().await;
                tx.send_modify(move |state| {
                    *state = std::mem::take(state).on_status(Instant::now(), event);
                });
            }
        });

        // create app and run it
        super::fancy_ui::run(super::fancy_ui::Params {
            terminal,
            begin: &params,
            child_state: rx,
            terminal_events: crossterm::event::EventStream::new(),
            log_paths: &log_paths,
        })
        .await?;
        debug!("Closing TUI");
    } else {
        debug!("Using simple TUI");
        super::simple_ui::run(super::simple_ui::Params {
            begin: &params,
            initial_info: &handle.initial_info,
            events: handle.events,
        })
        .await?;
    }

    Ok(())
}

impl Display for BeginParams {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Input: {}", self.input_file.to_string_lossy())?;
        if self.compression.is_identity() {
            writeln!(f, "  Size: {}", self.input_file_size)?;
        } else {
            writeln!(f, "  Size (compressed): {}", self.input_file_size)?;
        }
        writeln!(f, "  Compression: {}", self.compression)?;
        writeln!(f)?;

        writeln!(f, "Output: {}", self.target.name)?;
        writeln!(f, "  Model: {}", self.target.model)?;
        writeln!(f, "  Size: {}", self.target.size)?;
        writeln!(f, "  Block size: {}", self.target.block_size)?;
        writeln!(f, "  Type: {}", self.target.target_type)?;
        writeln!(f, "  Path: {}", self.target.devnode.to_string_lossy())?;

        if self.target.target_type == device::Type::Disk {
            writeln!(f, "  Removable: {}", self.target.removable)?;
        }

        Ok(())
    }
}
