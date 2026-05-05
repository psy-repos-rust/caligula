//! Exposes the [`Orchestrator`], which is a facade that orchestrates all "high-level" work
//! and tracks the state of worker tasks.

use crate::{
    herder_daemon::ipc::WriteVerifyEvent,
    herder_facade::{HerderFacade, StartWriterError},
};
use futures::StreamExt;
use std::{sync::Arc, time::Instant};

pub mod watch;
pub use self::write_verify::{WriteVerifyParams, WriteVerifyStarted, WriterState};

mod write_verify;

/// Main facade for UI implementations to interact with the rest of the program's logic.
///
/// This can be thought of as the glue between the UI and the backend, handling the following:
///
/// - spawning child processes and escalating them
/// - orchestrating multi-step workflows, such as write + verify
/// - reduction of event streams from the child processes into full states, along with [`Watch`]
///   handles for you to query state updates
///
/// Note that the interface is fully asynchronous. For synchronous UI implementations, you should
/// spawn a worker task as a shim between the [`Orchestrator`] and your synchronous UI threads,
/// probably using channels and such.
pub trait Orchestrator {
    /// Start a write + verify workflow.
    ///
    /// Returns when we get an initial success message from the task group, or there was a failure.
    fn start_write_verify(
        &self,
        escalate: bool,
        begin_params: WriteVerifyParams,
    ) -> impl Future<Output = Result<WriteVerifyStarted, StartWriterError<WriteVerifyEvent>>>;
}

/// Actual orchestrator implementation used by Caligula.
struct OrchestratorImpl<H> {
    h: Arc<tokio::sync::Mutex<H>>,
}

impl<H: HerderFacade> Orchestrator for OrchestratorImpl<H> {
    fn start_write_verify(
        &self,
        escalate: bool,
        params: WriteVerifyParams,
    ) -> impl Future<Output = Result<WriteVerifyStarted, StartWriterError<WriteVerifyEvent>>> {
        async move {
            // request the herder to start the action
            let mut lock = self.h.lock().await;
            let handle = lock
                .start_herd(params.make_child_config(), escalate)
                .await?;
            drop(lock);

            // create state reduction task
            let (tx_state, rx_state) = tokio::sync::watch::channel(WriterState::initial(
                Instant::now(),
                !params.compression.is_identity(),
                handle.initial_info.input_file_bytes,
            ));
            let mut events = handle.events;
            let _jh = tokio::spawn(async move {
                while !tx_state.borrow().is_finished() && !tx_state.is_closed() {
                    let event = events.next().await;
                    tx_state.send_modify(move |state| {
                        *state = std::mem::take(state).on_status(Instant::now(), event);
                    });
                }
            });
            let state = self::watch::Watch { rx: rx_state };

            Ok(WriteVerifyStarted {
                start: handle.initial_info,
                state,
            })
        }
    }
}

/// Make the actual prod-used orchestrator implementation.
pub fn make_orchestrator_impl(
    h: impl HerderFacade + Send + Sync + 'static,
) -> impl Orchestrator + Send + Sync + 'static {
    OrchestratorImpl {
        h: Arc::new(tokio::sync::Mutex::new(h)),
    }
}
