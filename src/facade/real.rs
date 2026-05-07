use std::{path::PathBuf, time::Instant};

use futures::StreamExt;

use super::legacy_facade::{DaemonError, LegacyFacade};
use crate::{
    escalation::EscalationMethod,
    facade::{
        Analyzer, DiskList, DiskWatcher, Escalator, Orchestrator, WriteVerifyParams,
        WriterVerifyState,
        analyze_input::InputAnalysis,
        watch::Watch,
        workflow::hash::{HashingState, StartHashParams},
    },
};

/// Actual CaligulaFacade implementation used by Caligula.
pub struct FacadeImpl<H> {
    inner: tokio::sync::Mutex<Inner<H>>,
}

struct Inner<H> {
    // TODO: get rid of the entire herder facade thing altogether. just assimilate the good parts
    // into CaligulaFacade.
    h: H,
    escalation: Option<Option<EscalationMethod>>,
}

impl<H> FacadeImpl<H> {
    pub fn new(h: H) -> Self {
        Self {
            inner: Inner {
                h,
                escalation: None,
            }
            .into(),
        }
    }
}

impl<H: LegacyFacade + Send + 'static> Orchestrator<WriteVerifyParams> for FacadeImpl<H> {
    async fn start_workflow(&self, params: WriteVerifyParams) -> Watch<WriterVerifyState> {
        tracing::info!("Requesting herder to start");

        // request the herder to start the action
        let mut inner = self.inner.lock().await;
        let esc = inner.escalation.is_some();
        let Ok(handle) = inner.h.start_herd(params.make_child_config(), esc).await else {
            todo!()
            /*return Watch {
                rx: tokio::sync::watch::channel(WriterVerifyState::error(todo!())).1,
            };*/
        };
        drop(inner);

        // create state reduction task
        let (tx_state, rx_state) = tokio::sync::watch::channel(WriterVerifyState::initial(
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
        super::watch::Watch { rx: rx_state }
    }
}

impl<H: LegacyFacade + Send + 'static> Orchestrator<StartHashParams> for FacadeImpl<H> {
    async fn start_workflow(&self, _workflow: StartHashParams) -> Watch<HashingState> {
        unimplemented!(
            "Until this is implemented, for testing purposes, you may replace this with test values."
        )
    }
}

impl<H: LegacyFacade> Escalator for FacadeImpl<H> {
    async fn escalate(&self, method: Option<EscalationMethod>) -> Result<(), DaemonError> {
        let mut inner = self.inner.lock().await;
        inner.escalation = Some(method);
        inner.h.ensure_escalated_daemon().await?;
        Ok(())
    }

    fn is_escalated(&self) -> bool {
        // TODO: this is badly implemented but it's good enough for writing new UIs
        // against. It will be improved when we get rid of herder facade.
        let Ok(lock) = self.inner.try_lock() else {
            return false;
        };
        lock.escalation.is_some()
    }
}

impl<H> DiskWatcher for FacadeImpl<H> {
    fn watch_disks(&self) -> Watch<DiskList> {
        unimplemented!(
            "Until this is implemented, for testing purposes, you may replace this with test values."
        )
    }
}

impl<H> Analyzer for FacadeImpl<H> {
    async fn analyze_input(&self, _input: PathBuf) -> std::io::Result<InputAnalysis> {
        unimplemented!(
            "Until this is implemented, for testing purposes, you may replace this with test values."
        )
    }
}
