use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::HerdAction;
use crate::{
    compression::CompressionFormat,
    device::Type,
    herder_api::error::{DiskError, InputFileError, IoError, UnmountError},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WriteVerifyAction {
    pub dest: PathBuf,
    pub src: PathBuf,
    pub verify: bool,
    pub compression: CompressionFormat,
    pub target_type: Type,
    pub block_size: Option<u64>,
}

impl HerdAction for WriteVerifyAction {
    type Event = WriteVerifyEvent;
    type Start = WriteVerifyStart;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WriteVerifyEvent {
    InitSuccess(WriteVerifyStart),
    TotalBytes {
        src: u64,
        dest: u64,
    },
    FinishedWriting {
        verifying: bool,
    },
    BlockSizeChanged(u64),
    BlockSizeSpeedInfo {
        blocks_written: usize,
        block_size: usize,
        duration_millis: u64,
    },
    Success,
    Error(LegacyWriteVerifyError),
}

super::impl_try_from_top_level_herd_event!(Writer => WriteVerifyEvent);

impl super::HerdEvent for WriteVerifyEvent {
    type Failure = LegacyWriteVerifyError;
    type StartInfo = WriteVerifyStart;

    fn downcast_as_initial_info(self) -> Result<Self::StartInfo, Self> {
        match self {
            WriteVerifyEvent::InitSuccess(e) => Ok(e),
            other => Err(other),
        }
    }

    fn downcast_as_failure(self) -> Result<Self::Failure, Self> {
        match self {
            WriteVerifyEvent::Error(e) => Ok(e),
            other => Err(other),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WriteVerifyStart {
    pub input_file_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
pub enum LegacyWriteVerifyError {
    #[error("Unexpected end of output file. Is your output file too small?")]
    EndOfOutput,
    #[error("Unknown error occurred in child process: {0}")]
    UnknownChildProcError(String),
    #[error("Failed to unmount disk: {0}")]
    FailedToUnmount(#[from] UnmountError),
    #[error("The child process unexpectedly terminated!")]
    UnexpectedTermination,
    #[error("Disk verification failed!")]
    VerificationFailed,
    #[error("Error handling input: {0}")]
    InputFile(#[from] IoError<InputFileError>),
    #[error("Error handling output: {0}")]
    OutputFile(#[from] IoError<DiskError>),
}
