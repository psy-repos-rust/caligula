use std::{
    io::{Read, Write},
    sync::{
        RwLock,
        atomic::{AtomicU32, Ordering},
    },
    time::{Duration, Instant},
};

use lockfree::queue::Queue;

pub struct JunctionTracker {
    /// Contains all transfers logged to this object since the last snapshot.
    ///
    /// Ironically, the "read" part of the lock is for the writers, while the
    /// "write" part of the lock is for the readers. The reason to do it
    /// this way is because the reader wants to get a consistent snapshot of
    /// the entire state of the operation, but multiple writers can write to
    /// the same thing at once.
    transfers: RwLock<Inner>,
    next_id: AtomicU32,
}

struct Inner {
    q: Queue<(u32, TransferStat)>,
    e: Option<std::io::Error>,
}

impl JunctionTracker {
    pub fn new() -> Self {
        Self {
            transfers: RwLock::new(Inner { q: Queue::new() }),
            next_id: 0.into(),
        }
    }

    /// Create a new [`Junction`] that logs its transfers to this
    /// [`JunctionStorage`].
    pub fn create<'a>(&'a self) -> Junction<'a> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        Junction { id, parent: self }
    }

    /// Take all logged transfers out of this [`JunctionStorage`].
    pub fn take_transfers(&self) -> Vec<(u32, TransferStat)> {
        let mut lock = self.transfers.write().unwrap();
        let values = std::mem::take(&mut *lock);
        drop(lock);

        values.pop_iter().collect()
    }
}

/// A [`Junction`] represents a point at which data is being transfered. Worker
/// threads can log [`TransferStat`]s, and the reader of the parent
/// [`JunctionStorage`] can get a consistent snapshot of the state of work.
#[derive(Clone)]
pub struct Junction<'a> {
    id: u32,
    parent: &'a JunctionTracker,
}

impl<'a> Junction<'a> {
    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn log(&self, stat: TransferStat) {
        self.parent.transfers.read().unwrap().push((self.id, stat));
    }
}

/// Statistics representing a single transfer of bytes.
///
/// Transfers usually represent syscalls or calls to [`Write`]/[`Read`].
#[derive(Debug, Clone, Copy)]
pub struct TransferStat {
    transfer_started: Instant,
    transfer_ended: Instant,
    bytes: u64,
}

impl TransferStat {
    pub fn new(transfer_started: Instant, transfer_ended: Instant, bytes: u64) -> Self {
        Self {
            transfer_started,
            transfer_ended,
            bytes,
        }
    }

    /// The relative time at which this operation started.
    pub fn started(&self) -> Instant {
        self.transfer_started
    }

    /// The relative time at which this operation ended.
    pub fn ended(&self) -> Instant {
        self.transfer_ended
    }

    /// How long this transfer took.
    pub fn duration(&self) -> Duration {
        self.ended() - self.started()
    }

    /// How many bytes were transferred.
    ///
    /// 0 represents a failure.
    pub fn bytes(&self) -> u64 {
        self.bytes
    }

    /// Whether this operation was successful or a failure.
    pub fn success(&self) -> bool {
        self.bytes != 0
    }
}

/// A [`Read`] that logs to a [`Junction`].
#[must_use]
pub struct ReadJunction<'a, R: Read> {
    read: R,
    junction: Junction<'a>,
}

impl<'a, R: Read> ReadJunction<'a, R> {
    pub fn new(read: R, junction: Junction<'a>) -> Self {
        Self { read, junction }
    }

    pub fn junction(&self) -> &Junction<'a> {
        &self.junction
    }
}

impl<'a, R: Read> Read for ReadJunction<'a, R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let start = Instant::now();
        let result = self.read.read(buf);
        let end = Instant::now();

        self.junction.log(TransferStat::new(
            start,
            end,
            result.as_ref().map(|x| *x as u64).unwrap_or(0),
        ));

        result
    }
}

/// A [`Write`] that logs to a [`Junction`].
#[must_use]
pub struct WriteJunction<'a, W: Write> {
    write: W,
    junction: Junction<'a>,
}

impl<'a, W: Write> WriteJunction<'a, W> {
    pub fn new(write: W, junction: Junction<'a>) -> Self {
        Self { write, junction }
    }

    pub fn junction(&self) -> &Junction<'a> {
        &self.junction
    }
}

impl<'a, W: Write> Write for WriteJunction<'a, W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let start = Instant::now();
        let result = self.write.write(buf);
        let end = Instant::now();

        self.junction.log(TransferStat::new(
            start,
            end,
            result.as_ref().map(|x| *x as u64).unwrap_or(0),
        ));

        result
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.write.flush()
    }
}
