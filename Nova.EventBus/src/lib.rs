//! Typed local IPC over loopback TCP. See the README and examples for usage.
mod broker;
mod client;
mod config;
mod protocol;
mod service;

pub use broker::Broker;
pub use client::{Client, ConnectionState, Subscription};
pub use config::Config;
pub use service::Service;

use serde::{Serialize, de::DeserializeOwned};
use std::time::Duration;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("I/O: {0}")]
    Io(#[from] std::io::Error),
    #[error("configuration: {0}")]
    Config(String),
    #[error("serialization: {0}")]
    Codec(String),
    #[error("protocol: {0}")]
    Protocol(String),
    #[error("connection lost; delivery or execution may already have occurred")]
    Disconnected,
    #[error("operation timed out; delivery or execution may already have occurred")]
    Timeout,
    #[error("remote error: {0}")]
    Remote(String),
    #[error("subscriber missed {0} events")]
    Lagged(u64),
    #[error("client has been closed")]
    Closed,
}

pub type Result<T> = std::result::Result<T, Error>;

pub(crate) fn encode<T: Serialize + ?Sized>(value: &T) -> Result<Vec<u8>> {
    rmp_serde::to_vec_named(value).map_err(|e| Error::Codec(e.to_string()))
}

pub(crate) fn decode<T: DeserializeOwned>(bytes: &[u8]) -> Result<T> {
    rmp_serde::from_slice(bytes).map_err(|e| Error::Codec(e.to_string()))
}

pub(crate) async fn timed<T>(ms: u64, future: impl Future<Output = Result<T>>) -> Result<T> {
    tokio::time::timeout(Duration::from_millis(ms), future)
        .await
        .map_err(|_| Error::Timeout)?
}
