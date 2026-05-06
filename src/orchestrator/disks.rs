#![expect(unused, reason = "Stub interface created for later use.")]

use std::{collections::VecDeque, iter, sync::Mutex};

use crate::device::{DeviceParseError, WriteTarget};

pub struct DiskList {
    /// List of disks we've discovered.
    disks: Vec<WriteTarget>,

    /// Errors encountered while listing disks.
    errors: Mutex<VecDeque<DeviceParseError>>,
}

impl DiskList {
    /// List of disks we've discovered.
    pub fn disks(&self) -> &[WriteTarget] {
        &self.disks
    }

    /// All errors encountered while listing disks, in order as encountered.
    ///
    /// Accessing this iterator will pop errors off this disk list.
    pub fn errors(&self) -> impl Iterator<Item = DeviceParseError> + '_ {
        let mut guard = self.errors.lock().unwrap();
        iter::from_fn(move || guard.pop_front())
    }
}
