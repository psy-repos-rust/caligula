use std::{sync::Arc, time::Instant};

use futures::StreamExt;

use super::herder_facade::{HerderFacade, StartWriterError};
use crate::{
    herder_api::write_verify::WriteVerifyEvent,
    orchestrator::{Orchestrator, WriteVerifyParams, WriteVerifyStarted, WriterState},
};

/// Actual orchestrator implementation used by Caligula.
pub struct OrchestratorImpl<H> {
    pub h: Arc<tokio::sync::Mutex<H>>,
}

impl<H: HerderFacade> Orchestrator for OrchestratorImpl<H> {
    async fn start_write_verify(
        &self,
        escalate: bool,
        params: WriteVerifyParams,
    ) -> Result<WriteVerifyStarted, StartWriterError<WriteVerifyEvent>> {
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
        let state = super::watch::Watch { rx: rx_state };

        Ok(WriteVerifyStarted {
            start: handle.initial_info,
            state,
        })
    }
}
