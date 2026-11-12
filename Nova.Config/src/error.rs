use std::path::PathBuf;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    #[error("I/O at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("invalid TOML at {path}: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: toml_edit::TomlError,
    },
    #[error("configuration type mismatch at {path}: {source}")]
    Decode {
        path: PathBuf,
        #[source]
        source: toml_edit::de::Error,
    },
    #[error("TOML serialization failed: {0}")]
    Encode(#[from] toml_edit::ser::Error),
    #[error("EventBus: {0}")]
    EventBus(String),
    #[error("configuration validation failed: {0}")]
    Validation(String),
    #[error("invalid library options: {0}")]
    Options(String),
    #[error("invalid key path {0:?}; use TOML dotted-key syntax")]
    InvalidKey(String),
    #[error("key not found: {0}")]
    MissingKey(String),
    #[error("cannot traverse a scalar or array at key path: {0}")]
    NotATable(String),
    #[error("value has no TOML representation (for example None)")]
    UnsupportedValue,
    #[error("file does not exist: {0}")]
    NotFound(PathBuf),
    #[error("creating a missing file requires defaults supplied to the builder")]
    MissingDefaults,
    #[error("configuration is read-only")]
    ReadOnly,
    #[error("local edits would be discarded; save or call discard_and_reload explicitly")]
    UnsavedChanges,
    #[error("configuration changed on disk: {0}; reload before saving")]
    Conflict(PathBuf),
    #[error("configuration exceeds {limit} bytes: {path}")]
    TooLarge { path: PathBuf, limit: usize },
    #[error("only regular files are supported (symbolic links are rejected): {0}")]
    NotRegularFile(PathBuf),
    #[error("configuration is not UTF-8: {0}")]
    InvalidUtf8(PathBuf),
}

impl From<nova_eventbus::Error> for Error {
    fn from(error: nova_eventbus::Error) -> Self {
        Self::EventBus(error.to_string())
    }
}

pub(crate) fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Error {
    Error::Io {
        path: path.into(),
        source,
    }
}
