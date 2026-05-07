use std::{path::PathBuf, sync::Arc, time::Instant};

use bytes::Bytes;
use sha2::Sha256;
use tokio::sync::{oneshot, watch};

use crate::{
    byteseries::ByteSeries,
    compression::CompressionFormat,
    facade::{
        watch::Watch,
        workflow::{Workflow, WorkflowState},
    },
    hash::HashAlg,
    io_graph::{
        BufNode, FileNode, ForwardWorker, GraphContext, HashWorker, JunctionTracker, Node,
        TransferStat, Worker as _,
    },
};

/// Parameters for starting a new hashing operation.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct HashWorkflow {
    /// File to use
    pub file: PathBuf,

    /// Algorithm to run
    pub alg: HashAlg,

    /// How to decompress the file before performing hash (if at all).
    pub compression: CompressionFormat,
}

impl Workflow for HashWorkflow {
    type State = HashingState;
}

/// Active, point-in-time state of a hashing operation.
pub struct HashingState {
    /// Read speed history.
    read_bytes_history: ByteSeries,
    /// How big the file is
    file_size_bytes: u64,
    /// Result of the operation. If [`None`], the operation is not yet finished.
    result: Option<std::io::Result<Bytes>>,
}

impl HashingState {
    fn new(now: Instant, file_size_bytes: u64) -> Self {
        Self {
            read_bytes_history: ByteSeries::new(now),
            file_size_bytes,
            result: None,
        }
    }

    fn failed(now: Instant, error: std::io::Error) -> Self {
        Self {
            read_bytes_history: ByteSeries::new(now),
            file_size_bytes: 0,
            result: Some(error),
        }
    }

    pub fn read_bytes_history(&self) -> &ByteSeries {
        &self.read_bytes_history
    }

    pub fn file_size_bytes(&self) -> u64 {
        self.file_size_bytes
    }
}

impl WorkflowState for HashingState {
    type Error = std::io::Error;
    type Success = Bytes;

    fn result(&self) -> Option<&Result<Self::Success, Self::Error>> {
        self.result.as_ref()
    }
}

async fn run(params: HashWorkflow) -> Watch<HashingState> {
    let state = Arc::new((GraphContext::new(), JunctionTracker::new()));

    let (tx_start, rx_start) = oneshot::channel();
    let (tx_end, rx_end) = oneshot::channel();

    let thread = std::thread::spawn({
        let state = state.clone();
        let params = params.clone();
        move || {
            let (ctx, js) = state.as_ref();
            run_thread(&params, tx_start, tx_end, ctx, js)
        }
    });

    let start = match rx_start.await.expect("thread panicked!") {
        Ok(r) => r,
        Err(e) => {
            let (_tx, rx) = watch::channel(HashingState::failed(Instant::now(), e));
            return Watch { rx };
        }
    };

    let (tx, rx) = watch::channel(HashingState::new(Instant::now(), start.size));
    tokio::task::spawn_local(async move {
        let (ctx, js) = state.as_ref();
    });

    Watch { rx }
}

struct StartData {
    size: u64,
    hasher_input_junction: u32,
}

fn run_thread(
    wf: &HashWorkflow,
    tx_start: oneshot::Sender<std::io::Result<StartData>>,
    tx_end: oneshot::Sender<std::io::Result<Bytes>>,
    ctx: &GraphContext,
    js: &JunctionTracker,
) {
    std::thread::scope(move |s| {
        let setup = (|| {
            let file = FileNode::new(wf.file.clone(), js.create())?;
            let buf = BufNode::new(65536, js.create(), js.create());
            let file_info = file.info();
            let _buf_info = buf.info();

            let start_data = StartData {
                size: file_info.extra.1,
                hasher_input_junction: buf.output.junction().id(),
            };

            let read = ForwardWorker::new(4096, file.output, buf.input);
            let hash = HashWorker::<Sha256, _>::new(4096, buf.output);

            Ok::<_, std::io::Error>((start_data, read, hash))
        })();

        let (read, hash) = match setup {
            Ok((start_data, read, hash)) => {
                let Ok(()) = tx_start.send(Ok(start_data)) else {
                    return;
                };
                (read, hash)
            }
            Err(err) => {
                tx_start.send(Err(err)).ok();
                return;
            }
        };

        let r = s.spawn(move || read.run(ctx));
        let h = s.spawn(move || hash.run(ctx));

        let r = r.join().unwrap();
        let h = h.join().unwrap();

        tx_end.send(r.and(h)).ok();
    })
}
