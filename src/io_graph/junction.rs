use std::{
    sync::{
        RwLock,
        atomic::{AtomicU32, Ordering},
    },
    time::{Duration, Instant},
};

use lockfree::queue::Queue;

pub struct JunctionStorage {
    /// Contains all transfers logged to this object since the last snapshot.
    ///
    /// Ironically, the "read" part of the lock is for the writers, while the "write"
    /// part of the lock is for the readers. The reason to do it this way is because the
    /// reader wants to get a consistent snapshot of the entire state of the operation,
    /// but multiple writers can write to the same thing at once.
    transfers: RwLock<Queue<(u32, TransferStat)>>,
    next_id: AtomicU32,
}

impl JunctionStorage {
    pub fn new() -> Self {
        Self {
            transfers: RwLock::new(Queue::new()),
            next_id: 0.into(),
        }
    }

    /// Create a new [`Junction`] that logs its transfers to this [`JunctionStorage`].
    pub fn create<'a>(&'a self) -> Junction<'a> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        Junction { id, parent: self }
    }

    pub fn take_transfers(&self) -> Vec<(u32, TransferStat)> {
        let mut lock = self.transfers.write().unwrap();
        let values = std::mem::take(&mut *lock);
        drop(lock);

        values.pop_iter().collect()
    }
}

/// A [`Junction`] represents a point at which data is being transfered. Worker threads can
/// log [`TransferStat`]s, and the reader of the parent [`JunctionStorage`] can get a consistent
/// snapshot of the state of work.
pub struct Junction<'a> {
    id: u32,
    parent: &'a JunctionStorage,
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
/// Transfers usually represent syscalls or calls to [`std::io::Write`]/[`std::io::Read`].
#[derive(Debug, Clone, Copy)]
pub struct TransferStat {
    transfer_started_ns: u64,
    transfer_ended_ns: u64,
    bytes: u64,
}

impl TransferStat {
    pub fn new(
        work_started: Instant,
        transfer_started: Instant,
        transfer_ended: Instant,
        bytes: u64,
    ) -> Self {
        Self {
            transfer_started_ns: (transfer_started - work_started).as_nanos() as u64,
            transfer_ended_ns: (transfer_ended - work_started).as_nanos() as u64,
            bytes,
        }
    }

    /// The time started, with respect to when the entire operation started.
    pub fn time_started(&self) -> Duration {
        Duration::from_nanos(self.transfer_started_ns)
    }

    /// The time ended, with respect to when the entire operation started.
    pub fn time_ended(&self) -> Duration {
        Duration::from_nanos(self.transfer_ended_ns)
    }

    /// How long this transfer took.
    pub fn duration(&self) -> Duration {
        self.time_ended() - self.time_started()
    }

    /// How many bytes were transferred.
    pub fn bytes(&self) -> u64 {
        self.bytes
    }
}
