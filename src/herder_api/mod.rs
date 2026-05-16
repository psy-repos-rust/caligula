//! Defines traits and modules for IPC between the child process and parent
//! process.
//!
//! The UI may speak some of these types, but it should prefer to use the
//! higher-level interfaces defined in [`crate::facade`].

pub mod client;
pub mod error;
pub mod write_verify;

use std::{error::Error, fmt::Debug};

use futures::stream::BoxStream;
use serde::{Deserialize, Serialize, de::DeserializeOwned};

pub struct HerderResponse<A: HerderAction, E> {
    pub start: A::Start,

    /// Outer [`Result`] represents transport failures, inner [`Result`]
    /// represents application-level failures.
    #[expect(clippy::type_complexity, reason = "ugh we'll improve this later")]
    pub events: BoxStream<'static, Result<Result<A::Event, A::Error>, E>>,
}

pub trait HerderService<A: HerderAction> {
    /// Errors that this transport may introduce.
    type Error: Error;

    /// Outer [`Result`] represents transport failures, inner [`Result`]
    /// represents application-level failures.
    async fn start(
        &self,
        action: A,
    ) -> Result<Result<HerderResponse<A, Self::Error>, A::Error>, Self::Error>;
}

/// Tell the herder to start a herd for performing an arbitrary action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StartHerd<A> {
    /// ID to associate with all of the herd's events
    pub id: u64,

    /// The action to perform
    pub action: A,
}

/// Arbitrary herd initialization action. This can be anything, from writing to
/// verifying to voiding.
pub trait HerderAction: Message {
    type Start: Message;

    type Error: Message + Error;

    /// The events emitted by the herd afterwards.
    type Event: Message;
}

/// Trait alias for things we can work with on the wire or in RPC.
pub trait Message:
    Serialize + DeserializeOwned + Debug + Clone + PartialEq + Send + 'static
{
}

impl<T: Serialize + DeserializeOwned + Debug + Clone + PartialEq + Send + 'static> Message for T {}
