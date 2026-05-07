use std::sync::Arc;

use crate::{
    escalation::EscalationMethod,
    facade::{CaligulaFacade, DaemonError, WriteVerifyParams, WriterVerifyState, watch::Watch},
    runtime::RemoteSpawn,
};

pub trait FacadeExt: CaligulaFacade {
    /// Like [`CaligulaFacade::start_write_verify()`], but it blocks your thread
    /// while waiting for it to start.
    ///
    /// THIS SHOULD ABSOLUTELY NOT UNDER ANY CIRCUMSTANCES be used in an async
    /// context! Just use the non-blocking version of the trait! It's mostly
    /// only useful for the simple UI wizard, which is inherently blocky.
    fn start_write_verify_blocking(
        self: Arc<Self>,
        spawn: impl RemoteSpawn,
        params: WriteVerifyParams,
    ) -> Watch<WriterVerifyState> {
        spawn
            .spawn(move || async move { self.start_workflow(params).await })
            .blocking_recv()
            .expect("remote task dropped!")
    }

    /// Like [`CaligulaFacade::escalate()`], but it blocks your thread while
    /// waiting for it to start.
    ///
    /// THIS SHOULD ABSOLUTELY NOT UNDER ANY CIRCUMSTANCES be used in an async
    /// context! Just use the non-blocking version of the trait! It's mostly
    /// only useful for the simple UI wizard, which is inherently blocky.
    fn escalate_blocking(
        self: Arc<Self>,
        spawn: impl RemoteSpawn,
        method: Option<EscalationMethod>,
    ) -> Result<(), DaemonError> {
        spawn
            .spawn(move || async move { self.escalate(method).await })
            .blocking_recv()
            .expect("remote task dropped!")
    }
}

impl<O: CaligulaFacade> FacadeExt for O {}
