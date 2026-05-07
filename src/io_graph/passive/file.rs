use std::{fs::File, io::BufReader, path::PathBuf};

use crate::io_graph::{
    Node, NodeInfo,
    junction::{Junction, ReadJunction},
};

pub struct FileNode<'a> {
    path: PathBuf,
    pub output: ReadJunction<'a, BufReader<File>>,
}

impl<'a> FileNode<'a> {
    pub fn new(path: PathBuf, junction: Junction<'a>) -> std::io::Result<Self> {
        Ok(Self {
            output: ReadJunction::new(BufReader::new(File::open(&path)?), junction),
            path,
        })
    }
}

impl<'a> Node<'a> for FileNode<'a> {
    type Info = PathBuf;

    fn info(&self) -> NodeInfo<'a, Self::Info> {
        NodeInfo {
            extra: self.path.clone(),
            inputs: vec![],
            outputs: vec![self.output.junction().clone()],
        }
    }
}
