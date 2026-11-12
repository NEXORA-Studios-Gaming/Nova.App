use crate::{ConfigFile, Error, Options, Result};
use nova_eventbus::{Client, Service};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::sync::Mutex;

type Shared = Arc<Mutex<ConfigFile<toml::Value>>>;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub eventbus_config: PathBuf,
    pub operation_options: PathBuf,
    pub configs: Vec<ConfigEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigEntry {
    pub id: String,
    pub path: PathBuf,
}

impl Manifest {
    pub fn load(path: impl AsRef<Path>) -> Result<(Self, PathBuf)> {
        let path = path.as_ref();
        let text = fs::read_to_string(path).map_err(|source| Error::Io {
            path: path.into(),
            source,
        })?;
        let manifest: Self = toml::from_str(&text)
            .map_err(|e| Error::Validation(format!("component manifest: {e}")))?;
        let base = path.parent().unwrap_or(Path::new("."));
        let base = if base.is_absolute() {
            base.to_path_buf()
        } else {
            std::env::current_dir()
                .map_err(|e| Error::Io {
                    path: base.into(),
                    source: e,
                })?
                .join(base)
        };
        Ok((manifest, base))
    }
}

pub struct ConfigComponent {
    files: HashMap<String, Shared>,
}

impl ConfigComponent {
    pub fn open(manifest: Manifest, base: &Path, options: Options) -> Result<Self> {
        if manifest.configs.is_empty() {
            return Err(Error::Validation(
                "manifest must register at least one config file".into(),
            ));
        }
        let mut files = HashMap::new();
        for entry in manifest.configs {
            validate_id(&entry.id)?;
            if files.contains_key(&entry.id) {
                return Err(Error::Validation(format!(
                    "duplicate config id: {}",
                    entry.id
                )));
            }
            let path = if entry.path.is_absolute() {
                entry.path
            } else {
                base.join(entry.path)
            };
            let file = ConfigFile::<toml::Value>::builder(path)
                .options(options.clone())
                .defaults(toml::Value::Table(toml::map::Map::new()))
                .validate(|value| {
                    if value.is_table() {
                        Ok(())
                    } else {
                        Err("root config value must be a TOML table".into())
                    }
                })
                .open()?;
            files.insert(entry.id, Arc::new(Mutex::new(file)));
        }
        Ok(Self { files })
    }

    pub fn register(&self, service: &mut Service, client: Client) -> Result<()> {
        let files = Arc::new(self.files.clone());
        let names = {
            let mut names: Vec<_> = self.files.keys().cloned().collect();
            names.sort();
            names
        };
        service.handle("list", move |_: Empty| {
            let names = names.clone();
            async move { Ok(names) }
        })?;

        let list_files = files.clone();
        service.handle("get", move |request: GetRequest| {
            let files = list_files.clone();
            async move {
                let file = files.get(&request.config).ok_or_else(|| {
                    nova_eventbus::Error::Remote(format!("unknown config: {}", request.config))
                })?;
                let file = file.lock().await;
                file.get::<toml::Value>(&request.key).map_err(bus_error)
            }
        })?;

        let set_files = files.clone();
        let events = client.clone();
        service.handle("set", move |request: SetRequest| {
            let files = set_files.clone();
            let events = events.clone();
            async move {
                let file = files.get(&request.config).ok_or_else(|| {
                    nova_eventbus::Error::Remote(format!("unknown config: {}", request.config))
                })?;
                let mut file = file.lock().await;
                let previous = file.get::<toml::Value>(&request.key).ok();
                file.set(&request.key, &request.value).map_err(bus_error)?;
                if let Err(error) = file.save() {
                    if let Some(previous) = previous {
                        let _ = file.set(&request.key, &previous);
                    } else {
                        let _ = file.remove(&request.key);
                    }
                    return Err(bus_error(error));
                }
                let change = Changed {
                    config: request.config,
                    key: request.key,
                };
                let _ = events.publish("config.changed", &change).await;
                Ok(true)
            }
        })?;

        let remove_files = files.clone();
        let events = client.clone();
        service.handle("remove", move |request: KeyRequest| {
            let files = remove_files.clone();
            let events = events.clone();
            async move {
                let file = files.get(&request.config).ok_or_else(|| {
                    nova_eventbus::Error::Remote(format!("unknown config: {}", request.config))
                })?;
                let mut file = file.lock().await;
                let previous = file.get::<toml::Value>(&request.key).ok();
                let removed = file.remove(&request.key).map_err(bus_error)?;
                if !removed {
                    return Ok(false);
                }
                if let Err(error) = file.save() {
                    if let Some(previous) = previous {
                        let _ = file.set(&request.key, &previous);
                    }
                    return Err(bus_error(error));
                }
                let change = Changed {
                    config: request.config,
                    key: request.key,
                };
                let _ = events.publish("config.changed", &change).await;
                Ok(true)
            }
        })?;

        let reload_files = files.clone();
        let events = client.clone();
        service.handle("reload", move |request: ConfigRequest| {
            let files = reload_files.clone();
            let events = events.clone();
            async move {
                let file = files.get(&request.config).ok_or_else(|| {
                    nova_eventbus::Error::Remote(format!("unknown config: {}", request.config))
                })?;
                file.lock().await.discard_and_reload().map_err(bus_error)?;
                let change = Changed {
                    config: request.config,
                    key: "*".into(),
                };
                let _ = events.publish("config.changed", &change).await;
                Ok(true)
            }
        })?;
        Ok(())
    }
}

fn validate_id(id: &str) -> Result<()> {
    if id.is_empty()
        || id.len() > 64
        || !id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    {
        return Err(Error::Validation(format!(
            "invalid config id {id:?}; use 1..=64 ASCII letters, digits, '_' or '-'"
        )));
    }
    Ok(())
}

fn bus_error(error: Error) -> nova_eventbus::Error {
    nova_eventbus::Error::Remote(error.to_string())
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Empty {}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigRequest {
    pub config: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetRequest {
    pub config: String,
    pub key: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyRequest {
    pub config: String,
    pub key: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetRequest {
    pub config: String,
    pub key: String,
    pub value: toml::Value,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Changed {
    pub config: String,
    pub key: String,
}
