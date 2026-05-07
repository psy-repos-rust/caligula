use std::{
    io::{self, Read},
    marker::PhantomData,
};

use bytes::Bytes;
use digest::Digest;

use crate::io_graph::{Node, NodeInfo, Worker, executor::GraphContext, junction::ReadJunction};

pub struct HashWorker<'a, H: Digest, R: Read> {
    input: ReadJunction<'a, R>,
    buffer: usize,
    _phantom: PhantomData<fn() -> H>,
}

impl<'a, H: Digest, R: Read + Send + 'a> HashWorker<'a, H, R> {
    pub fn new(buffer: usize, input: ReadJunction<'a, R>) -> Box<Self> {
        Box::new(Self {
            input,
            buffer,
            _phantom: PhantomData,
        })
    }
}

impl<'a, H: Digest, R: Read> Node<'a> for HashWorker<'a, H, R> {
    type Info = usize;

    fn info(&self) -> NodeInfo<'a, Self::Info> {
        NodeInfo {
            extra: self.buffer,
            inputs: vec![self.input.junction().clone()],
            outputs: vec![],
        }
    }
}

impl<'a, H: Digest + 'a, R: Read + Send + 'a> Worker<'a> for HashWorker<'a, H, R> {
    type Output = Bytes;

    fn run(mut self: Box<Self>, context: &GraphContext) -> io::Result<Bytes> {
        let mut h = H::new();
        let mut buf = vec![0u8; self.buffer];
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
