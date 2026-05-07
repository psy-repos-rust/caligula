use std::{
    io::{self, Read},
    marker::PhantomData,
    sync::Arc,
};

use bytes::Bytes;
use digest::Digest;

use crate::io_graph::{GraphContext, Node, NodeInfo, Worker, junction::ReadJunction};

pub struct HashWorker<'a, R: Read, H: Digest> {
    input: ReadJunction<'a, R>,
    buffer: usize,
    _phantom: PhantomData<fn() -> H>,
}

impl<'a, R: Read + Sync + 'a, H: Digest> HashWorker<'a, R, H> {
    pub fn new(input: ReadJunction<'a, R>, buffer: usize) -> Self {
        Self {
            input,
            buffer,
            _phantom: PhantomData,
        }
    }
}

impl<'a, R: Read, H: Digest> Node<'a> for HashWorker<'a, R, H> {
    type Info = usize;

    fn info(&self) -> NodeInfo<'a, Self::Info> {
        NodeInfo {
            extra: self.buffer,
            inputs: vec![self.input.junction().clone()],
            outputs: vec![],
        }
    }
}

impl<'a, R: Read + Sync + 'a, H: Digest + 'a> Worker<'a> for HashWorker<'a, R, H> {
    type Output = Bytes;

    fn run(mut self: Box<Self>, context: Arc<GraphContext>) -> io::Result<Bytes> {
        let mut h = H::new();
        let mut buf = vec![0u8; 4096];
        while !context.halt() {
            let count = self.input.read(&mut buf)?;

            if count == 0 {
                break;
            }

            h.update(&buf[..count]);
        }
        let h = h.finalize();
        Ok(Bytes::copy_from_slice(&h))
    }
}
