mod write_verify;

use std::sync::Arc;

pub use self::write_verify::{BeginParams, WriterState};
use crate::{
    herder_daemon::ipc::WriteVerifyEvent,
    herder_facade::{HerdHandle, HerderFacade, StartWriterError},
};

pub trait Orchestrator {
    fn start_write_verify(
        &self,
        escalate: bool,
        begin_params: BeginParams,
    ) -> impl Future<Output = Result<HerdHandle<WriteVerifyEvent>, StartWriterError<WriteVerifyEvent>>>;
}

struct OrchestratorImpl<H> {
    h: Arc<tokio::sync::Mutex<H>>,
}

impl<H: HerderFacade> Orchestrator for OrchestratorImpl<H> {
    fn start_write_verify(
        &self,
        escalate: bool,
        begin_params: BeginParams,
    ) -> impl Future<Output = Result<HerdHandle<WriteVerifyEvent>, StartWriterError<WriteVerifyEvent>>>
    {
        async move {
            let mut lock = self.h.lock().await;
            lock.start_herd(begin_params.make_child_config(), escalate)
                .await
        }
    }
}

pub fn make_orchestrator_impl(
    h: impl HerderFacade + Send + Sync + 'static,
) -> impl Orchestrator + Send + Sync + 'static {
    OrchestratorImpl {
        h: Arc::new(tokio::sync::Mutex::new(h)),
    }
}
