use std::io::{self, Read, Write};

use crate::io_graph::{
    Node, NodeInfo, Worker,
    executor::GraphContext,
    junction::{ReadJunction, WriteJunction},
};

pub struct ForwardWorker<'a, R: Read, W: Write> {
    input: ReadJunction<'a, R>,
    output: WriteJunction<'a, W>,
    buffer: usize,
}

impl<'a, R: Read + Send + 'a, W: Write + Send + 'a> ForwardWorker<'a, R, W> {
    pub fn new(
        buffer: usize,
        input: ReadJunction<'a, R>,
        output: WriteJunction<'a, W>,
    ) -> Box<Self> {
        Box::new(Self {
            input,
            output,
            buffer,
        })
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

impl<'a, R: Read + Send + 'a, W: Write + Send + 'a> Worker<'a> for ForwardWorker<'a, R, W> {
    type Output = ();

    fn run(mut self: Box<Self>, context: &GraphContext) -> io::Result<()> {
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
