use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use serde::{Serialize, de::DeserializeOwned};

use crate::io_graph::junction::Junction;

mod active;
mod junction;
mod passive;

/// A [`Node`] is an object in the I/O graph connected to other objects.
#[must_use]
pub trait Node<'a> {
    /// Extra information this node adds to a [`NodeInfo`].
    type Info: Serialize + DeserializeOwned + 'static;

    /// Generate the information about this node.
    fn info(&self) -> NodeInfo<'a, Self::Info>;
}

/// A [`Node`] representing an active worker thread.
#[must_use]
pub trait Worker<'a>: Node<'a> + Sync + 'a {
    /// Final, successful value computed by this [`Worker`].
    type Output: Send + 'static;

    fn run(self: Box<Self>, context: Arc<GraphContext>) -> std::io::Result<Self::Output>;
}

/// Information associated with a [`Node`]
pub struct NodeInfo<'a, T> {
    pub extra: T,
    pub inputs: Vec<Junction<'a>>,
    pub outputs: Vec<Junction<'a>>,
}

pub struct GraphContext {
    halt: AtomicBool,
}

impl GraphContext {
    pub fn new() -> Self {
        Self { halt: false.into() }
    }

    pub fn halt(&self) -> bool {
        self.halt.load(Ordering::Relaxed)
    }
}
