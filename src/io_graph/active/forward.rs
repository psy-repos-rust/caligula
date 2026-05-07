use std::{
    io::{self, Read, Write},
    sync::Arc,
};

use crate::io_graph::{
    GraphContext, Node, NodeInfo, Worker,
    junction::{ReadJunction, WriteJunction},
};

pub struct ForwardWorker<'a, R: Read, W: Write> {
    input: ReadJunction<'a, R>,
    output: WriteJunction<'a, W>,
    buffer: usize,
}

impl<'a, R: Read + Sync + 'a, W: Write + Sync + 'a> ForwardWorker<'a, R, W> {
    pub fn new(input: ReadJunction<'a, R>, output: WriteJunction<'a, W>, buffer: usize) -> Self {
        Self {
            input,
            output,
            buffer,
        }
    }
}

impl<'a, R: Read, W: Write> Node<'a> for ForwardWorker<'a, R, W> {
    type Info = usize;

    fn info(&self) -> NodeInfo<'a, Self::Info> {
        NodeInfo {
            extra: self.buffer,
            inputs: vec![self.input.junction().clone()],
            outputs: vec![self.input.junction().clone()],
        }
    }
}

impl<'a, R: Read + Sync + 'a, W: Write + Sync + 'a> Worker<'a> for ForwardWorker<'a, R, W> {
    type Output = ();

    fn run(mut self: Box<Self>, context: Arc<GraphContext>) -> io::Result<()> {
        let mut buf = vec![0u8; self.buffer];
        while !context.halt() {
            let count = self.input.read(&mut buf)?;
            if count == 0 {
                break;
            }

            self.output.write_all(&buf[..count])?;
        }
        Ok(())
    }
}
