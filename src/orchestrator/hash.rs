#![expect(unused, reason = "Stub interface created for later use.")]

use bytes::Bytes;
use std::{path::PathBuf, time::Instant};

use crate::{
    byteseries::ByteSeries, compression::CompressionFormat, hash::HashAlg,
    orchestrator::watch::Watch,
};

/// Parameters for starting a new hashing operation.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct StartHashParams {
    /// File to use
    pub file: PathBuf,

    /// Algorithm to run
    pub alg: HashAlg,

    /// How to decompress the file before performing hash (if at all).
    pub compression: CompressionFormat,
}

/// Result from hash starting.
pub struct HashStarted {
    /// Handle for watching the state updates of this hasher.
    pub state: Watch<HashingState>,
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
    pub fn new(now: Instant, file_size_bytes: u64) -> Self {
        Self {
            read_bytes_history: ByteSeries::new(now),
            file_size_bytes,
            result: None,
        }
    }

    /// Whether or not this operation is finished.
    pub fn is_finished(&self) -> bool {
        self.result.is_some()
    }

    pub fn read_bytes_history(&self) -> &ByteSeries {
        &self.read_bytes_history
    }

    pub fn file_size_bytes(&self) -> u64 {
        self.file_size_bytes
    }

    pub fn result(&self) -> Option<&std::io::Result<Bytes>> {
        self.result.as_ref()
    }
}
