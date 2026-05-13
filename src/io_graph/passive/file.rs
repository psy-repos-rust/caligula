use std::{fs::File, io::BufReader, os::fd::AsRawFd, path::PathBuf};

use nix::fcntl::PosixFadviseAdvice;
use tracing::warn;
use tracing_unwrap::ResultExt;

use crate::io_graph::ReadReceiver;

pub struct FileNode {
    path: PathBuf,
    size: u64,
    pub output: ReadReceiver<BufReader<File>>,
}

impl FileNode {
    pub fn new(path: PathBuf, read_size: usize) -> std::io::Result<Self> {
        let f = File::open(&path)?;
        let size = f.metadata()?.len();
        
        nix::fcntl::posix_fadvise(&f, 0, 0, PosixFadviseAdvice::POSIX_FADV_SEQUENTIAL).ok_or_log();

        let br = BufReader::with_capacity(read_size, f);
        Ok(Self {
            output: ReadReceiver::new(br, read_size),
            size,
            path,
        })
    }

    pub fn size(&self) -> u64 {
        self.size
    }
}
