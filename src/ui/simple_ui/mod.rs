//! The "simple UI". This is module holds subroutines that don't use ratatui,
//! and don't take up the entire terminal screen.
//!
//! As pretty as ratatui looks, sometimes you can't use a full-featured terminal.
//! This is what this module is for.

use self::ask_hash::ask_hash;
use self::ask_outfile::ask_compression;
use self::ask_outfile::ask_outfile;
use self::ask_outfile::confirm_write;
use super::cli::BurnArgs;
use crate::{
    device::WriteTarget,
    herder_api::write_verify::{WriteVerifyError, WriteVerifyEvent, WriteVerifyStart},
    orchestrator::{
        Orchestrator, OrchestratorExt, StartWriterError, WriteVerifyParams, WriteVerifyStarted,
        WriterState, watch::Watch,
    },
    runtime::RemoteSpawn,
    ui::cli::UseSudo,
};
use indicatif::ProgressBar;
use indicatif::ProgressStyle;
use inquire::Confirm;
use std::sync::Arc;
use std::time::Duration;
use tracing::debug;

mod ask_hash;
mod ask_outfile;

/// How often we refresh the display
const REFRESH_PERIOD: Duration = Duration::from_millis(250);

/// Run the simple UI setup wizard, a cruel being that interrogates the user until it is
/// satisfied with its answers.
///
/// Returns the [BeginParams] if the user confirms, and None if the user doesn't.
#[tracing::instrument(skip_all)]
pub fn do_setup_wizard(args: &BurnArgs) -> Result<Option<WriteVerifyParams>, anyhow::Error> {
    let compression = ask_compression(args)?;
    let _hash_info = ask_hash(args, compression)?;
    let target = match &args.out {
        Some(f) => WriteTarget::try_from(f.as_ref())?,
        None => ask_outfile(args)?,
    };
    let begin_params = WriteVerifyParams::new(args.image.clone(), compression, target)?;
    if !confirm_write(args, &begin_params)? {
        eprintln!("Aborting.");
        return Ok(None);
    }
    Ok(Some(begin_params))
}

pub struct Params<'a> {
    pub initial_info: &'a WriteVerifyStart,
    pub child_state: Watch<WriterState>,
}

/// Attempt to start burning the disk with the given params.
///
/// If received permission denied, figures out if it needs to ask the user to sudo
/// based on what's provided in the `root` argument.
#[tracing::instrument(skip_all, fields(root, interactive))]
pub fn try_start_write_or_escalate(
    orc: Arc<impl Orchestrator>,
    runtime: &impl RemoteSpawn,
    args: &WriteVerifyParams,
    root: UseSudo,
    interactive: bool,
) -> Result<WriteVerifyStarted, StartWriterError<WriteVerifyEvent>> {
    tracing::info!("Starting burn without escalation");

    let err = match orc
        .clone()
        .start_write_verify_blocking(runtime, args.clone())
    {
        Ok(p) => {
            return Ok(p);
        }
        Err(e) => e,
    };

    if let StartWriterError::Failed(WriteVerifyError::PermissionDenied) = &err {
        tracing::info!("Unescalated burn failed");
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
                    orc.clone().escalate_blocking(runtime, None)?;
                    return Ok(orc.start_write_verify_blocking(runtime, args.clone())?);
                }
            }
            (UseSudo::Always, _) => {
                orc.clone().escalate_blocking(runtime, None)?;
                return Ok(orc.start_write_verify_blocking(runtime, args.clone())?);
            }
            _ => {}
        }
    }

    Err(err.into())
}

/// Run the simple TUI.
#[tracing::instrument(skip_all)]
pub async fn run<'a>(params: Params<'a>) -> anyhow::Result<()> {
    let input_file_bytes = params.initial_info.input_file_bytes;
    let write_progress = ProgressBar::new(100).with_message("Burning").with_style(
        ProgressStyle::with_template(
            "[{elapsed_precise}] {msg:>10} {wide_bar:.green/black} {percent:>3}%",
        )
        .unwrap(),
    );
    let verify_progress = ProgressBar::new(100).with_message("Verifying").with_style(
        ProgressStyle::with_template(
            "[{elapsed_precise}] {msg:>10} {wide_bar:.blue/black} {percent:>3}%",
        )
        .unwrap(),
    );

    let mut interval = tokio::time::interval(REFRESH_PERIOD);
    loop {
        interval.tick().await;

        let child_state = params.child_state.borrow();
        match &*child_state {
            WriterState::Writing(b) => {
                write_progress.set_position((b.approximate_ratio() * 1000.0) as u64)
            }
            WriterState::Verifying {
                total_write_bytes, ..
            } => verify_progress.set_position(total_write_bytes * 1000 / input_file_bytes),
            WriterState::Finished { .. } => break,
        }
    }
    println!("Done!");
    Ok(())
}
