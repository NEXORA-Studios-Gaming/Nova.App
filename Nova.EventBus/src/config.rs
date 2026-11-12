use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, path::Path};

/// Startup settings. Changes to the TOML file require restarting the process.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub endpoint: SocketAddr,
    pub connect_timeout_ms: u64,
    pub request_timeout_ms: u64,
    pub write_timeout_ms: u64,
    pub reconnect_delay_ms: u64,
    pub max_frame_bytes: usize,
    pub queue_capacity: usize,
    pub max_connections: usize,
    pub max_in_flight_requests: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            endpoint: SocketAddr::from(([127, 0, 0, 1], 47631)),
            connect_timeout_ms: 2000,
            request_timeout_ms: 5000,
            write_timeout_ms: 5000,
            reconnect_delay_ms: 500,
            max_frame_bytes: 8 * 1024 * 1024,
            queue_capacity: 256,
            max_connections: 128,
            max_in_flight_requests: 64,
        }
    }
}

impl Config {
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let text = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&text).map_err(|e| Error::Config(e.to_string()))?;
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<()> {
        if !self.endpoint.ip().is_loopback() {
            return Err(Error::Config("endpoint must be a loopback address".into()));
        }
        if self.max_frame_bytes < 128 || self.max_frame_bytes > 64 * 1024 * 1024 {
            return Err(Error::Config(
                "max_frame_bytes must be in 128..=67108864".into(),
            ));
        }
        if [
            self.queue_capacity,
            self.max_connections,
            self.max_in_flight_requests,
        ]
        .iter()
        .any(|&n| n == 0 || n > 65536)
        {
            return Err(Error::Config(
                "queue and concurrency limits must be in 1..=65536".into(),
            ));
        }
        if [
            self.connect_timeout_ms,
            self.request_timeout_ms,
            self.write_timeout_ms,
            self.reconnect_delay_ms,
        ]
        .contains(&0)
        {
            return Err(Error::Config(
                "timeouts and reconnect delay must be positive".into(),
            ));
        }
        Ok(())
    }
}
