use crate::io_graph::{GraphContext, RecvBytes, SendBytes, Worker};

pub struct ForwardWorker<Rx: RecvBytes, Tx: SendBytes> {
    input: Rx,
    output: Tx,
}

impl<Rx: RecvBytes + Send, Tx: SendBytes + Send> ForwardWorker<Rx, Tx> {
    pub fn new(input: Rx, output: Tx) -> Box<Self> {
        Box::new(Self { input, output })
    }
}

impl<Rx: RecvBytes + Send, Tx: SendBytes + Send> Worker for ForwardWorker<Rx, Tx> {
    type Error = std::io::Error;
    type Output = ();

    fn run(mut self: Box<Self>, context: &GraphContext) -> Result<Self::Output, Self::Error> {
        while !context.halt() {
            let bytes = self.input.recv()?;
            match bytes {
                Some(b) => self.output.send(b)?,
                None => break,
            }
        }

        Ok(())
    }
}
