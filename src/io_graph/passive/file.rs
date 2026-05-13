use std::{fs::File, io::BufReader, path::PathBuf};

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
        Ok(Self {
            output: ReadReceiver::new(BufReader::new(f), read_size),
            size,
            path,
        })
    }

    pub fn size(&self) -> u64 {
        self.size
    }
}
