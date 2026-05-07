use std::error::Error;

use crate::facade::watch::Watch;

pub mod hash;
pub mod write_verify;

/// An object that can handle the spawning of a specific workflow.
pub trait Orchestrator<W: Workflow> {
    /// Start the workflow in the background.
    ///
    /// The workflow may have immediately returned an error. It is up to the
    /// caller to check and see if that is the case.
    async fn start_workflow(&self, workflow: W) -> Watch<W::State>;
}

/// Marker trait representing the parameters of a workflow that can be scheduled
/// to run in the background with an [`Orchestrator`].
pub trait Workflow {
    /// The state associated with this [`Workflow`].
    type State: WorkflowState;
}

/// A point-in-time representation of the state of an initialized [`Workflow`].
pub trait WorkflowState: Sync + 'static {
    /// Result of this workflow on success.
    type Success: Sync + 'static;

    /// Result of this workflow on failure.
    type Error: Sync + Error + 'static;

    /// Check this workflow for a result. If it returns [`None`], then this
    /// workflow is still running.
    fn result(&self) -> Option<&Result<Self::Success, Self::Error>>;
}
