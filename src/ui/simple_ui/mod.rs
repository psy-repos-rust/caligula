//! The "simple UI". This is module holds subroutines that don't use ratatui,
//! and don't take up the entire terminal screen.
//!
//! As pretty as ratatui looks, sometimes you can't use a full-featured terminal.
//! This is what this module is for.

use std::time::Instant;

use futures::Stream;
use futures::StreamExt;
use indicatif::ProgressBar;
use indicatif::ProgressStyle;

use crate::device::WriteTarget;
use crate::herder_daemon::ipc::WriteVerifyEvent;
use crate::herder_daemon::ipc::WriteVerifyStart;
use crate::orchestrator::BeginParams;
use crate::orchestrator::WriterState;

use self::ask_hash::ask_hash;
use self::ask_outfile::ask_compression;
use self::ask_outfile::ask_outfile;
use self::ask_outfile::confirm_write;

use super::cli::BurnArgs;

mod ask_hash;
mod ask_outfile;

/// Returns the [BeginParams] if the user confirms, and None if the user doesn't.
#[tracing::instrument(skip_all)]
pub fn do_setup_wizard(args: &BurnArgs) -> Result<Option<BeginParams>, anyhow::Error> {
    let compression = ask_compression(args)?;
    let _hash_info = ask_hash(args, compression)?;
    let target = match &args.out {
        Some(f) => WriteTarget::try_from(f.as_ref())?,
        None => ask_outfile(args)?,
    };
    let begin_params = BeginParams::new(args.image.clone(), compression, target)?;
    if !confirm_write(args, &begin_params)? {
        eprintln!("Aborting.");
        return Ok(None);
    }
    Ok(Some(begin_params))
}

pub struct Params<'a, S>
where
    S: Stream<Item = WriteVerifyEvent> + Unpin + 'a,
{
    pub begin: &'a BeginParams,
    pub initial_info: &'a WriteVerifyStart,
    pub events: S,
}

/// Run the simple TUI.
#[tracing::instrument(skip_all)]
pub async fn run<'a, S>(params: Params<'a, S>) -> anyhow::Result<()>
where
    S: Stream<Item = WriteVerifyEvent> + Unpin + 'a,
{
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

    let mut child_state = WriterState::initial(
        Instant::now(),
        !params.begin.compression.is_identity(),
        input_file_bytes,
    );
    let mut events = params.events;

    loop {
        let x = events.next().await;
        child_state = child_state.on_status(Instant::now(), x);
        match &child_state {
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
