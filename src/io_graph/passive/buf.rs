use std::{
    io::{Read, Write},
    sync::mpsc,
};

use bytes::{Buf, BufMut, Bytes, BytesMut};
use ringbuf::{HeapCons, HeapProd, HeapRb, traits::Split};

use crate::io_graph::{
    Node, NodeInfo,
    junction::{Junction, ReadJunction, WriteJunction},
};

pub struct BufNode<'a> {
    pub input: WriteJunction<'a, MpscWrite>,
    pub output: ReadJunction<'a, MpscRead>,
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

pub struct MpscRead {
    current: Bytes,

    /// rx handle to sender end. Recv 0 bytes to signal EOF.
    rx: mpsc::Receiver<Bytes>,
}

impl Read for MpscRead {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        // see if we need to read from the buffer
        if self.current.is_empty() {
            let msg = self.rx.recv().map_err(|_| {
                std::io::Error::new(std::io::ErrorKind::BrokenPipe, "mpsc sender was dropped")
            })?;

            // 0 len means close
            if msg.len() == 0 {
                return Ok(0);
            }
            self.current = msg;
        }

        Ok(match self.current.try_copy_to_slice(buf) {
            Ok(()) => buf.len(), // all successfully copied
            Err(err) => err.available,
        })
    }
}

pub struct MpscWrite {
    tx_limit: usize,
    buf: BytesMut,

    /// tx handle to receiver end. Send 0 bytes to signal EOF.
    tx: mpsc::SyncSender<Bytes>,
}

impl MpscWrite {
    fn close(self) {
        self.tx.send(Bytes::new()).ok();
    }
}

impl Write for MpscWrite {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        // calculate how many bytes we are writing
        let max_write = self.tx_limit - self.buf.len();
        let actual_write = buf.len().min(max_write);

        // append to buffer
        self.buf.put(&buf[..actual_write]);

        // flush if we hit the limit
        if self.buf.len() >= self.tx_limit {
            self.flush()?;
        }

        Ok(actual_write)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        let msg = std::mem::take(&mut self.buf).freeze();

        // don't send a 0, because that tells the rx that we're done
        if msg.len() == 0 {
            return Ok(());
        }

        self.tx.send(msg).map_err(|_| {
            std::io::Error::new(std::io::ErrorKind::BrokenPipe, "mpsc receiver was dropped")
        });
        Ok(())
    }
}
