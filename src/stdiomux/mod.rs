//! Utilities for multiplexed streamed RPC requests.

use std::error::Error;

use bytes::Bytes;
use futures::stream::BoxStream;

pub mod client;
pub mod server;
mod util;

/// A service that, when called with a stream of bytes, returns another stream
/// of bytes.
pub trait BytestreamService {
    /// Error this service may return.
    type Error: Error;

    /// Call this service.
    fn call(
        &self,
        req: BoxStream<'static, Bytes>,
    ) -> BoxStream<'static, Result<Bytes, Self::Error>>;
}
