use crate::{Error, Result, storage};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictPolicy {
    /// Refuse to replace a file whose bytes differ from the last load/save.
    #[default]
    Reject,
    /// Explicitly allow replacing externally edited contents.
    Overwrite,
}

/// Operation settings, supplied in Rust or loaded from a separate TOML file.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Options {
    pub create_if_missing: bool,
    pub create_parent_dirs: bool,
    pub read_only: bool,
    pub conflict_policy: ConflictPolicy,
    pub max_file_bytes: usize,
    /// Sync temporary-file contents before replacement; not a directory fsync.
    pub sync_on_save: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            create_if_missing: false,
            create_parent_dirs: false,
            read_only: false,
            conflict_policy: ConflictPolicy::Reject,
            max_file_bytes: 1024 * 1024,
            sync_on_save: true,
        }
    }
}

impl Options {
    /// Options files are limited to 64 KiB and must already exist.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let bytes = storage::read(path, 64 * 1024)?.ok_or_else(|| Error::NotFound(path.into()))?;
        let text = std::str::from_utf8(&bytes).map_err(|_| Error::InvalidUtf8(path.into()))?;
        let options: Self = toml_edit::de::from_str(text).map_err(|source| Error::Decode {
            path: path.into(),
            source,
        })?;
        options.validate()?;
        Ok(options)
    }

    pub fn validate(&self) -> Result<()> {
        if !(1..=64 * 1024 * 1024).contains(&self.max_file_bytes) {
            return Err(Error::Options(
                "max_file_bytes must be in 1..=67108864".into(),
            ));
        }
        if self.read_only && self.create_if_missing {
            return Err(Error::Options(
                "read_only and create_if_missing cannot both be true".into(),
            ));
        }
        Ok(())
    }
}
