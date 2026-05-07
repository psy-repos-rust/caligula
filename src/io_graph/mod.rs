use serde::{Serialize, de::DeserializeOwned};

pub use crate::io_graph::{
    active::{forward::ForwardWorker, hash::HashWorker},
    executor::GraphContext,
    junction::{Junction, JunctionTracker, TransferStat},
    passive::{buf::BufNode, file::FileNode},
};

mod active;
mod executor;
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
pub trait Worker<'a>: Node<'a> + Send + 'a {
    /// Final, successful value computed by this [`Worker`].
    type Output: Send + 'static;

    fn run(self: Box<Self>, context: &GraphContext) -> std::io::Result<Self::Output>;
}

/// Information associated with a [`Node`]
pub struct NodeInfo<'a, T> {
    pub extra: T,
    pub inputs: Vec<Junction<'a>>,
    pub outputs: Vec<Junction<'a>>,
}

/// Information associated with a [`Node`]
pub struct NodeData<T> {
    pub extra: T,
    pub inputs: Vec<u32>,
    pub outputs: Vec<u32>,
}
