//! Typed TOML files with comment-preserving edits and explicit persistence.
//!
//! ```no_run
//! use nova_config::{ConfigFile, Options};
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Default, Serialize, Deserialize)]
//! #[serde(default)]
//! struct Settings { volume: u8 }
//!
//! let mut file = ConfigFile::<Settings>::builder("settings.toml")
//!     .options(Options { create_if_missing: true, ..Options::default() })
//!     .defaults(Settings { volume: 80 })
//!     .validate(|s| if s.volume <= 100 { Ok(()) } else { Err("volume exceeds 100".into()) })
//!     .open()?;
//! file.set("volume", &50u8)?;
//! file.save()?;
//! # Ok::<(), nova_config::Error>(())
//! ```
mod component;
mod document;
mod error;
mod file;
mod options;
mod storage;

pub use component::{
    Changed, ConfigComponent, ConfigEntry, ConfigRequest, Empty, GetRequest, KeyRequest, Manifest,
    SetRequest,
};
pub use error::{Error, Result};
pub use file::{ConfigBuilder, ConfigFile};
pub use options::{ConflictPolicy, Options};
