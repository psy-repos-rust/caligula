use futures::{Stream, StreamExt as _, stream};
use ratatui::{Terminal, prelude::Backend};

use self::state::UIEvent;
use crate::{
    herder_daemon::ipc::{WriteVerifyEvent, WriteVerifyStart},
    logging::LogPaths,
    orchestrator::BeginParams,
    ui::fancy_ui::{display::FancyUI, state::State},
};
use std::{sync::Arc, time::Duration, time::Instant};

mod display;
mod state;
mod widgets;

pub struct FancyUiParams<'a, B, C, T>
where
    B: Backend + 'a,
    C: Stream<Item = WriteVerifyEvent> + 'a,
    T: Stream<Item = std::io::Result<crossterm::event::Event>> + 'a,
{
    pub terminal: &'a mut Terminal<B>,
    pub begin: &'a BeginParams,
    pub initial_info: WriteVerifyStart,
    pub child_events: C,
    pub terminal_events: T,
    pub log_paths: Arc<LogPaths>,
}

/// Run the fancy TUI.
#[tracing::instrument(skip_all)]
pub async fn run<'a, B, C, T>(params: FancyUiParams<'a, B, C, T>) -> anyhow::Result<()>
where
    B: Backend,
    C: Stream<Item = WriteVerifyEvent> + 'a,
    T: Stream<Item = std::io::Result<crossterm::event::Event>> + 'a,
{
    let child_events = params
        .child_events
        .map(|e: WriteVerifyEvent| UIEvent::RecvChildStatus(Instant::now(), Some(e)))
        .chain(stream::once(async {
            UIEvent::RecvChildStatus(Instant::now(), None)
        }));
    let terminal_events =
        params
            .terminal_events
            .map(|e: std::io::Result<crossterm::event::Event>| {
                UIEvent::RecvTermEvent(e.map_err(|e| (e.to_string(), e.kind())))
            });
    let timeout_events = stream::unfold(
        tokio::time::interval(Duration::from_millis(250)),
        |mut i| async move {
            i.tick().await;
            Some((UIEvent::SleepTimeout, i))
        },
    );
    let events = Box::pin(stream::select(
        stream::select(child_events, terminal_events),
        timeout_events,
    ));

    let input_file_bytes = params.initial_info.input_file_bytes;

    let ui = FancyUI {
        terminal: params.terminal,
        events,
        state: State::initial(Instant::now(), params.begin, input_file_bytes),
        log_paths: params.log_paths,
    };

    ui.show().await
}
