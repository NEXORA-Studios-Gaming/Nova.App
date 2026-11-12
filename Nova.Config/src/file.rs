use crate::{Error, Options, Result, document, error::io, storage};
use serde::{Serialize, de::DeserializeOwned};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use toml_edit::DocumentMut;

type Validator<T> = Arc<dyn Fn(&T) -> std::result::Result<(), String> + Send + Sync>;

/// Builder for a configuration file. No file I/O occurs before `open`.
pub struct ConfigBuilder<T> {
    path: PathBuf,
    options: Options,
    defaults: Option<T>,
    validator: Option<Validator<T>>,
}

impl<T: Serialize + DeserializeOwned> ConfigBuilder<T> {
    pub fn options(mut self, options: Options) -> Self {
        self.options = options;
        self
    }

    /// Used only when creating a missing file. For missing fields use Serde defaults.
    pub fn defaults(mut self, defaults: T) -> Self {
        self.defaults = Some(defaults);
        self
    }

    /// The validator must be pure; it can run more than once for an operation.
    pub fn validate(
        mut self,
        validator: impl Fn(&T) -> std::result::Result<(), String> + Send + Sync + 'static,
    ) -> Self {
        self.validator = Some(Arc::new(validator));
        self
    }

    pub fn open(self) -> Result<ConfigFile<T>> {
        self.options.validate()?;
        // Capture the current directory so later cwd changes cannot redirect a save.
        let path = if self.path.is_absolute() {
            self.path
        } else {
            std::env::current_dir()
                .map_err(|e| io(&self.path, e))?
                .join(self.path)
        };
        let baseline = storage::read(&path, self.options.max_file_bytes)?;
        let (document, create) = match baseline.as_deref() {
            Some(bytes) => (parse(&path, bytes)?, false),
            None if self.options.create_if_missing => {
                let defaults = self.defaults.ok_or(Error::MissingDefaults)?;
                (toml_edit::ser::to_document(&defaults)?, true)
            }
            None => return Err(Error::NotFound(path)),
        };
        let data = decode(&path, &document)?;
        let baseline_render = document.to_string();
        let mut file = ConfigFile {
            path,
            options: self.options,
            document,
            data,
            baseline,
            baseline_render,
            validator: self.validator,
        };
        file.check(&file.data)?;
        file.check_size(&file.document)?;
        if create {
            file.save()?;
        }
        Ok(file)
    }
}

/// One process owns writes; readers may use their own instances.
///
/// Edits are transactional in memory. `save` explicitly persists them; drop never saves.
/// Share a single instance behind a mutex when multiple threads need to edit it.
pub struct ConfigFile<T> {
    path: PathBuf,
    options: Options,
    document: DocumentMut,
    data: T,
    baseline: Option<Vec<u8>>,
    baseline_render: String,
    validator: Option<Validator<T>>,
}

impl<T: Serialize + DeserializeOwned> ConfigFile<T> {
    pub fn builder(path: impl AsRef<Path>) -> ConfigBuilder<T> {
        ConfigBuilder {
            path: path.as_ref().into(),
            options: Options::default(),
            defaults: None,
            validator: None,
        }
    }

    /// Opens an existing file with default operation settings.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        Self::builder(path).open()
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn options(&self) -> &Options {
        &self.options
    }
    pub fn data(&self) -> &T {
        &self.data
    }
    pub fn to_toml(&self) -> String {
        self.render(&self.document)
    }
    pub fn is_dirty(&self) -> bool {
        self.baseline.as_deref() != Some(self.to_toml().as_bytes())
    }

    /// Reads a typed value using TOML dotted-key syntax. Arrays are read as whole values.
    pub fn get<V: DeserializeOwned>(&self, path: &str) -> Result<V> {
        let keys = document::keys(path)?;
        let item = document::get(self.document.as_item(), &keys, path)?;
        document::decode_value(item).map_err(|source| Error::Decode {
            path: self.path.clone(),
            source,
        })
    }

    /// Edits a value in memory, then decodes and validates the entire configuration.
    pub fn set<V: Serialize + ?Sized>(&mut self, path: &str, value: &V) -> Result<()> {
        self.writable()?;
        let keys = document::keys(path)?;
        let mut candidate = self.document.clone();
        document::set(
            candidate.as_item_mut(),
            &keys,
            document::value(value)?,
            path,
        )?;
        self.commit(candidate)
    }

    /// Removes a key if the resulting configuration remains valid.
    /// Returns false if absent. Empty containing tables are retained.
    pub fn remove(&mut self, path: &str) -> Result<bool> {
        self.writable()?;
        let keys = document::keys(path)?;
        let mut candidate = self.document.clone();
        let removed = document::remove(candidate.as_item_mut(), &keys, path)?;
        if removed {
            self.commit(candidate)?;
        }
        Ok(removed)
    }

    /// Changes multiple typed fields as one validated in-memory transaction.
    /// Unknown keys and unchanged fields retain their original source representation.
    pub fn update(&mut self, edit: impl FnOnce(&mut T)) -> Result<()> {
        self.writable()?;
        let mut data: T = decode(&self.path, &self.document)?;
        let before = toml_edit::ser::to_document(&data)?;
        edit(&mut data);
        self.check(&data)?;
        let after = toml_edit::ser::to_document(&data)?;
        let mut candidate = self.document.clone();
        document::merge_changes(candidate.as_item_mut(), before.as_item(), after.as_item());
        self.commit(candidate)
    }

    /// Saves via a temporary file in the same directory, with conflict detection by default.
    /// Failure retains local edits for inspection/retry. A clean save performs no I/O.
    pub fn save(&mut self) -> Result<()> {
        self.writable()?;
        if !self.is_dirty() {
            return Ok(());
        }
        let text = self.to_toml();
        storage::save(
            &self.path,
            text.as_bytes(),
            self.baseline.as_deref(),
            &self.options,
        )?;
        self.baseline_render = self.document.to_string();
        self.baseline = Some(text.into_bytes());
        Ok(())
    }

    /// Reloads a clean instance. Invalid files leave all in-memory state unchanged.
    pub fn reload(&mut self) -> Result<()> {
        if self.is_dirty() {
            return Err(Error::UnsavedChanges);
        }
        self.discard_and_reload()
    }

    /// Explicitly discards local edits, but only after the disk file passes all checks.
    pub fn discard_and_reload(&mut self) -> Result<()> {
        let baseline = storage::read(&self.path, self.options.max_file_bytes)?
            .ok_or_else(|| Error::NotFound(self.path.clone()))?;
        let document = parse(&self.path, &baseline)?;
        let data = decode(&self.path, &document)?;
        self.check(&data)?;
        self.baseline_render = document.to_string();
        self.document = document;
        self.data = data;
        self.baseline = Some(baseline);
        Ok(())
    }

    fn writable(&self) -> Result<()> {
        if self.options.read_only {
            Err(Error::ReadOnly)
        } else {
            Ok(())
        }
    }

    fn check(&self, data: &T) -> Result<()> {
        if let Some(validator) = &self.validator {
            validator(data).map_err(Error::Validation)?;
        }
        Ok(())
    }

    fn render(&self, document: &DocumentMut) -> String {
        let rendered = document.to_string();
        if let Some(baseline) = &self.baseline {
            if rendered == self.baseline_render {
                // TOML's parser normalizes CRLF, but a no-op must retain original bytes.
                return String::from_utf8_lossy(baseline).into_owned();
            }
            let first_newline = baseline.iter().position(|&byte| byte == b'\n');
            if first_newline.is_some_and(|index| index > 0 && baseline[index - 1] == b'\r') {
                return rendered.replace("\r\n", "\n").replace('\n', "\r\n");
            }
        }
        rendered
    }

    fn check_size(&self, document: &DocumentMut) -> Result<()> {
        if self.render(document).len() > self.options.max_file_bytes {
            return Err(Error::TooLarge {
                path: self.path.clone(),
                limit: self.options.max_file_bytes,
            });
        }
        Ok(())
    }

    fn commit(&mut self, document: DocumentMut) -> Result<()> {
        self.check_size(&document)?;
        let data = decode(&self.path, &document)?;
        self.check(&data)?;
        self.document = document;
        self.data = data;
        Ok(())
    }
}

fn parse(path: &Path, bytes: &[u8]) -> Result<DocumentMut> {
    let text = std::str::from_utf8(bytes).map_err(|_| Error::InvalidUtf8(path.into()))?;
    text.parse().map_err(|source| Error::Parse {
        path: path.into(),
        source,
    })
}

fn decode<T: DeserializeOwned>(path: &Path, document: &DocumentMut) -> Result<T> {
    toml_edit::de::from_document(document.clone()).map_err(|source| Error::Decode {
        path: path.into(),
        source,
    })
}
