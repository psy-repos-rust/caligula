//! Exposes the [`Orchestrator`], which is a facade that orchestrates all "high-level" work
//! and tracks the state of worker tasks.

use crate::herder_api::write_verify::*;
use std::sync::Arc;

mod herder_facade;
mod real;
pub mod watch;
pub use self::{
    herder_facade::StartWriterError,
    write_verify::{WriteVerifyParams, WriteVerifyStarted, WriterState},
};

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
    async fn start_write_verify(
        &self,
        escalate: bool,
        begin_params: WriteVerifyParams,
    ) -> Result<WriteVerifyStarted, StartWriterError<WriteVerifyEvent>>;
}

/// Make the actual prod-used orchestrator implementation.
pub fn make_orchestrator_impl(log_path: &str) -> impl Orchestrator + Send + Sync + 'static {
    self::real::OrchestratorImpl {
        h: Arc::new(tokio::sync::Mutex::new(
            herder_facade::make_herder_facade_impl(log_path),
        )),
    }
}
