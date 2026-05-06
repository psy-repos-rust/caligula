mod cli;
mod fancy_ui;
mod simple_ui;
mod utils;

use std::{fs::File, path::Path, sync::Arc};

pub use self::cli::BurnArgs;
pub use self::utils::ByteSpeed;
use crate::{
    logging::LogPaths,
    orchestrator::{WriteVerifyParams, WriteVerifyStarted, make_orchestrator_impl},
    runtime::RemoteSpawn,
    tty::TermiosRestore,
    ui::{cli::Interactive, simple_ui::do_setup_wizard, utils::TUICapture},
};
use tracing::{debug, info};

/// Entrypoint for TUI-based UIs.
pub fn main(_state_dir: &Path, log_paths: Arc<LogPaths>, args: BurnArgs) -> anyhow::Result<()> {
    let runtime = crate::runtime::AsyncRuntime::start();
    let orc = Arc::new(make_orchestrator_impl(log_paths.main()));

    let _termios_restore = match File::open("/dev/tty") {
        Ok(tty) => TermiosRestore::new(tty).ok(),
        Err(error) => {
            info!(
                ?error,
                "failed to open /dev/tty, will not attempt to restore after program"
            );
            None
        }
    };

    let Some(begin_params) = do_setup_wizard(&args)? else {
        return Ok(());
    };

    let handle = simple_ui::try_start_write_or_escalate(
        orc.clone(),
        &runtime,
        &begin_params,
        args.root,
        args.interactive.is_interactive(),
    )?;

    begin_writing(runtime, args.interactive, begin_params, handle, log_paths)?;

    debug!("Done!");
    Ok(())
}

pub fn begin_writing(
    runtime: impl RemoteSpawn,
    interactive: Interactive,
    params: WriteVerifyParams,
    started: WriteVerifyStarted,
    log_paths: Arc<LogPaths>,
) -> anyhow::Result<()> {
    if interactive.is_interactive() {
        let mut tui = TUICapture::new()?;
        let terminal = tui.terminal();
        // create app and run it
        fancy_ui::run(
            runtime,
            fancy_ui::Params {
                terminal,
                begin: &params,
                child_state: started.state,
                terminal_events: crossterm::event::EventStream::new(),
                log_paths: &log_paths,
            },
        );
        Ok(())
    } else {
        runtime
            .spawn(move || async move {
                simple_ui::run(simple_ui::Params {
                    initial_info: &started.start,
                    child_state: started.state,
                })
                .await
            })
            .blocking_recv()
            .expect("runtime failed")
    }
}
