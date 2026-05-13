use std::sync::mpsc;

use bytes::Bytes;

use crate::io_graph::{RecvBytes, SendBytes};

pub struct BufNode {
    pub input: MpscWrite,
    pub output: MpscRead,
}

impl BufNode {
    pub fn new(count: usize) -> Self {
        let (tx, rx) = mpsc::sync_channel(count);
        Self {
            input: MpscWrite { tx },
            output: MpscRead { rx },
        }
    }
}

pub struct MpscRead {
    rx: mpsc::Receiver<Bytes>,
}

impl RecvBytes for MpscRead {
    fn recv(&mut self) -> std::io::Result<Option<Bytes>> {
        // see if we need to read from the buffer
        let msg = self.rx.recv().map_err(|_| {
            std::io::Error::new(std::io::ErrorKind::BrokenPipe, "mpsc sender was dropped")
        })?;

        // 0 len means close
        if msg.is_empty() {
            return Ok(None);
        }

        Ok(Some(msg))
    }
}

pub struct MpscWrite {
    /// tx handle to receiver end. Send 0 bytes to signal EOF.
    tx: mpsc::SyncSender<Bytes>,
}

impl SendBytes for MpscWrite {
    fn send(&mut self, bytes: Bytes) -> std::io::Result<()> {
        if bytes.is_empty() {
            // don't send 0 bytes because that signals close
            return Ok(());
        }

        self._send(bytes)?;
        Ok(())
    }

    fn close(mut self) -> std::io::Result<()> {
        self._send(Bytes::new())?;
        Ok(())
    }
}

impl MpscWrite {
    fn _send(&mut self, bytes: Bytes) -> std::io::Result<()> {
        self.tx.send(bytes).map_err(|_| {
            std::io::Error::new(std::io::ErrorKind::BrokenPipe, "mpsc receiver was dropped")
        })?;
        Ok(())
    }
}
