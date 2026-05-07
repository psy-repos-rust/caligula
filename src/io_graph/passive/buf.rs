use ringbuf::{HeapCons, HeapProd, HeapRb, traits::Split};

use crate::io_graph::{
    Node, NodeInfo,
    junction::{Junction, ReadJunction, WriteJunction},
};

pub struct BufNode<'a> {
    pub input: WriteJunction<'a, HeapProd<u8>>,
    pub output: ReadJunction<'a, HeapCons<u8>>,
    size: usize,
}

impl<'a> BufNode<'a> {
    pub fn new(size: usize, input: Junction<'a>, output: Junction<'a>) -> Self {
        let (write, read) = HeapRb::new(size).split();
        Self {
            size,
            input: WriteJunction::new(write, input),
            output: ReadJunction::new(read, output),
        }
    }
}

impl<'a> Node<'a> for BufNode<'a> {
    type Info = usize;

    fn info(&self) -> NodeInfo<'a, Self::Info> {
        NodeInfo {
            extra: self.size,
            inputs: vec![self.input.junction().clone()],
            outputs: vec![self.output.junction().clone()],
        }
    }
}
