use nova_config::{ConfigFile, ConflictPolicy, Error, Options};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};
use tempfile::TempDir;

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
struct Settings {
    name: String,
    enabled: bool,
    window: Window,
    labels: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    note: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
struct Window {
    width: u32,
    height: u32,
}
impl Default for Window {
    fn default() -> Self {
        Self {
            width: 800,
            height: 600,
        }
    }
}

const SOURCE: &str = "# application settings\nname  = 'NOVA' # keep name comment\nenabled = true\nlabels = [\n  'alpha', # first\n  'beta',\n]\n\n# display settings\n[window]\nwidth  = 800 # horizontal pixels\nheight = 600\nunknown = 'preserve me' # not modeled by Rust\n\n[\"plugin.name\"]\n\"key.with.dot\" = 7\n";

fn fixture(text: &str) -> (TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.toml");
    fs::write(&path, text).unwrap();
    (dir, path)
}

fn load(path: &Path) -> ConfigFile<Settings> {
    ConfigFile::open(path).unwrap()
}

#[test]
fn edits_keep_comments_spacing_and_unknown_fields() {
    let (_dir, path) = fixture(SOURCE);
    let mut config = load(&path);
    assert!(!config.is_dirty());
    config.set("window.width", &1280u32).unwrap();
    let expected = SOURCE.replace("width  = 800 #", "width  = 1280 #");
    assert_eq!(config.to_toml(), expected);
    assert_eq!(config.data().window.width, 1280);
    assert_eq!(fs::read_to_string(&path).unwrap(), SOURCE);
    assert!(config.is_dirty());
    config.save().unwrap();
    assert_eq!(fs::read_to_string(&path).unwrap(), expected);
    assert!(!config.is_dirty());
    assert_eq!(fs::read_dir(path.parent().unwrap()).unwrap().count(), 1);
}

#[test]
fn typed_updates_change_only_modified_fields() {
    let (_dir, path) = fixture(SOURCE);
    let mut config = load(&path);
    config.update(|data| data.window.height = 900).unwrap();
    assert_eq!(
        config.to_toml(),
        SOURCE.replace("height = 600", "height = 900")
    );
    config.update(|_| {}).unwrap();
    assert!(!config.to_toml().contains("note ="));
    assert!(config.to_toml().contains("unknown = 'preserve me'"));
}

#[test]
fn quoted_keys_inline_tables_and_new_tables() {
    let (_dir, path) = fixture(
        "name='NOVA'\ninline = { value = 1 } # retain\n[\"plugin.name\"]\n\"key.with.dot\"=7\n",
    );
    let mut config = load(&path);
    assert_eq!(
        config
            .get::<u32>("\"plugin.name\".\"key.with.dot\"")
            .unwrap(),
        7
    );
    config.set("\"plugin.name\".\"key.with.dot\"", &8).unwrap();
    config.set("inline.value", &2).unwrap();
    config.set("nested.child.enabled", &true).unwrap();
    assert!(config.get::<bool>("nested.child.enabled").unwrap());
    assert!(config.to_toml().contains("inline = { value = 2 } # retain"));
    config.save().unwrap();
    assert_eq!(load(&path).get::<i64>("inline.value").unwrap(), 2);
    config
        .set(
            "inline.value",
            &Window {
                width: 100,
                height: 50,
            },
        )
        .unwrap();
    config.save().unwrap();
    assert_eq!(
        load(&path).get::<Window>("inline.value").unwrap().width,
        100
    );
    assert!(matches!(
        config.set("name.child", &1),
        Err(Error::NotATable(_))
    ));
    assert!(matches!(
        config.get::<String>("missing"),
        Err(Error::MissingKey(_))
    ));
    assert!(matches!(
        config.get::<String>("window..width"),
        Err(Error::InvalidKey(_))
    ));
}

#[test]
fn defaults_are_explicit_and_missing_fields_use_serde() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nested/settings.toml");
    assert!(matches!(
        ConfigFile::<Settings>::open(&path),
        Err(Error::NotFound(_))
    ));
    let options = Options {
        create_if_missing: true,
        create_parent_dirs: true,
        ..Options::default()
    };
    assert!(matches!(
        ConfigFile::<Settings>::builder(&path)
            .options(options.clone())
            .open(),
        Err(Error::MissingDefaults)
    ));
    let config = ConfigFile::builder(&path)
        .options(options)
        .defaults(Settings {
            name: "created".into(),
            ..Settings::default()
        })
        .open()
        .unwrap();
    assert_eq!(config.data().name, "created");
    assert!(!config.is_dirty());
    assert!(path.exists());
    fs::write(&path, "name = 'partial'\n").unwrap();
    assert_eq!(load(&path).data().window.width, 800);
    assert!(!load(&path).to_toml().contains("width"));
}

#[test]
fn validation_and_type_errors_are_transactional() {
    let (_dir, path) = fixture(SOURCE);
    let mut config = ConfigFile::<Settings>::builder(&path)
        .validate(|data| {
            if data.window.width >= data.window.height {
                Ok(())
            } else {
                Err("width must be >= height".into())
            }
        })
        .open()
        .unwrap();
    assert!(matches!(
        config.set("window.width", &100),
        Err(Error::Validation(_))
    ));
    assert!(matches!(
        config.set("enabled", &"wrong type"),
        Err(Error::Decode { .. })
    ));
    assert_eq!(config.to_toml(), SOURCE);
    assert!(!config.is_dirty());
    config
        .update(|data| {
            data.window.width = 400;
            data.window.height = 300;
        })
        .unwrap();
    assert_eq!(config.data().window.width, 400);
    let before = config.to_toml();
    assert!(config.update(|data| data.window.height = 500).is_err());
    assert_eq!(config.to_toml(), before);
}

#[test]
fn remove_and_optional_values() {
    let (_dir, path) = fixture("name = 'NOVA'\nnote = 'removable' # removed with value\n");
    let mut config = load(&path);
    config.update(|data| data.note = None).unwrap();
    assert!(!config.to_toml().contains("note"));
    assert!(!config.remove("unknown.child").unwrap());
    assert!(config.remove("name").unwrap());
    assert_eq!(config.data().name, "");
    assert!(matches!(
        config.set("note", &None::<String>),
        Err(Error::UnsupportedValue)
    ));
    #[derive(Serialize, Deserialize)]
    struct Required {
        name: String,
    }
    let mut required = ConfigFile::<Required>::open(&path).unwrap();
    assert!(matches!(required.remove("name"), Err(Error::Decode { .. })));
    assert!(required.to_toml().contains("name"));
}

#[test]
fn external_edits_cause_conflict_and_can_be_reloaded() {
    let (_dir, path) = fixture(SOURCE);
    let mut config = load(&path);
    config.set("name", &"local").unwrap();
    let external = SOURCE.replace("'NOVA'", "'external'");
    fs::write(&path, &external).unwrap();
    assert!(matches!(config.save(), Err(Error::Conflict(_))));
    assert_eq!(fs::read_to_string(&path).unwrap(), external);
    assert!(config.is_dirty());
    assert!(matches!(config.reload(), Err(Error::UnsavedChanges)));
    config.discard_and_reload().unwrap();
    assert_eq!(config.data().name, "external");
    assert!(!config.is_dirty());
    config.set("name", &"edited").unwrap();
    fs::remove_file(&path).unwrap();
    assert!(matches!(config.save(), Err(Error::Conflict(_))));
    assert!(!path.exists());
}

#[test]
fn overwrite_policy_is_explicit() {
    let (_dir, path) = fixture(SOURCE);
    let mut config = ConfigFile::<Settings>::builder(&path)
        .options(Options {
            conflict_policy: ConflictPolicy::Overwrite,
            ..Options::default()
        })
        .open()
        .unwrap();
    config.set("name", &"local").unwrap();
    fs::write(&path, "name='external'\n").unwrap();
    config.save().unwrap();
    assert_eq!(load(&path).data().name, "local");
}

#[test]
fn failed_reload_keeps_last_valid_state_and_local_edits() {
    let (_dir, path) = fixture(SOURCE);
    let mut config = load(&path);
    fs::write(&path, "not valid [").unwrap();
    assert!(matches!(config.reload(), Err(Error::Parse { .. })));
    assert_eq!(config.to_toml(), SOURCE);
    assert_eq!(config.data().name, "NOVA");
    config.set("name", &"local").unwrap();
    assert!(config.discard_and_reload().is_err());
    assert_eq!(config.data().name, "local");
    assert!(config.is_dirty());
    fs::write(&path, "name='new'\n").unwrap();
    config.discard_and_reload().unwrap();
    assert_eq!(config.data().name, "new");
}

#[test]
fn read_only_instances_can_reload_but_cannot_edit() {
    let (_dir, path) = fixture(SOURCE);
    let mut config = ConfigFile::<Settings>::builder(&path)
        .options(Options {
            read_only: true,
            ..Options::default()
        })
        .open()
        .unwrap();
    assert!(matches!(config.set("name", &"x"), Err(Error::ReadOnly)));
    assert!(matches!(config.remove("name"), Err(Error::ReadOnly)));
    assert!(matches!(
        config.update(|data| data.name.clear()),
        Err(Error::ReadOnly)
    ));
    assert!(matches!(config.save(), Err(Error::ReadOnly)));
    fs::write(&path, "name='new'\n").unwrap();
    config.reload().unwrap();
    assert_eq!(config.data().name, "new");
}

#[test]
fn bounded_reads_writes_and_invalid_utf8() {
    let (_dir, path) = fixture("name='short'\n");
    let options = Options {
        max_file_bytes: 64,
        ..Options::default()
    };
    let mut config = ConfigFile::<Settings>::builder(&path)
        .options(options.clone())
        .open()
        .unwrap();
    assert!(matches!(
        config.set("name", &"x".repeat(100)),
        Err(Error::TooLarge { .. })
    ));
    assert!(!config.is_dirty());
    fs::write(&path, "#".repeat(65)).unwrap();
    assert!(matches!(
        ConfigFile::<Settings>::builder(&path)
            .options(options)
            .open(),
        Err(Error::TooLarge { .. })
    ));
    fs::write(&path, [0xff, 0xfe]).unwrap();
    assert!(matches!(
        ConfigFile::<Settings>::open(&path),
        Err(Error::InvalidUtf8(_))
    ));
}

#[test]
fn operation_options_load_and_validate() {
    let (_dir, path) =
        fixture("create_if_missing = true\nmax_file_bytes = 2048\nconflict_policy = 'reject'\n");
    let options = Options::from_file(&path).unwrap();
    assert!(options.create_if_missing);
    assert_eq!(options.max_file_bytes, 2048);
    assert!(options.sync_on_save);
    fs::write(&path, "typo = true\n").unwrap();
    assert!(Options::from_file(&path).is_err());
    assert!(
        Options {
            max_file_bytes: 0,
            ..Options::default()
        }
        .validate()
        .is_err()
    );
    assert!(
        Options {
            read_only: true,
            create_if_missing: true,
            ..Options::default()
        }
        .validate()
        .is_err()
    );
}

#[test]
fn dropping_does_not_save_and_noop_keeps_source_bytes() {
    let source = "# header\r\nname = 'NOVA'\r\n";
    let (_dir, path) = fixture(source);
    let mut config = load(&path);
    assert_eq!(config.to_toml(), source);
    assert!(!config.is_dirty());
    config.save().unwrap();
    config.set("name", &"saved").unwrap();
    let saved = "# header\r\nname = \"saved\"\r\n";
    assert_eq!(config.to_toml(), saved);
    config.save().unwrap();
    assert_eq!(fs::read_to_string(&path).unwrap(), saved);
    assert!(!config.is_dirty());
    config.set("name", &"unsaved").unwrap();
    drop(config);
    assert_eq!(fs::read_to_string(&path).unwrap(), saved);
}

#[cfg(unix)]
#[test]
fn file_modes_survive_save_and_symlinks_are_rejected() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let (dir, path) = fixture(SOURCE);
    fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();
    let mut config = load(&path);
    config.set("name", &"updated").unwrap();
    config.save().unwrap();
    assert_eq!(
        fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o640
    );
    let link = dir.path().join("link.toml");
    symlink(&path, &link).unwrap();
    assert!(matches!(
        ConfigFile::<Settings>::open(&link),
        Err(Error::NotRegularFile(_))
    ));
}
