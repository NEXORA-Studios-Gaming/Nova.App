use nova_config::{ConfigFile, Options, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
struct Settings {
    window: Window,
    audio: Audio,
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
            width: 1280,
            height: 720,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
struct Audio {
    volume: u8,
}
impl Default for Audio {
    fn default() -> Self {
        Self { volume: 80 }
    }
}

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let path = args.next().unwrap_or_else(|| "demo/settings.toml".into());
    let options_path = args.next().unwrap_or_else(|| "nova-config.toml".into());
    let mut config = ConfigFile::<Settings>::builder(path)
        .options(Options::from_file(options_path)?)
        .defaults(Settings::default())
        .validate(|data| {
            if data.audio.volume > 100 {
                return Err("audio.volume must be <= 100".into());
            }
            if data.window.width == 0 || data.window.height == 0 {
                return Err("window size must be positive".into());
            }
            Ok(())
        })
        .open()?;

    println!("Loaded: {:?}", config.data());
    config.set("audio.volume", &75u8)?;
    config.update(|data| {
        data.window.width = 1600;
        data.window.height = 900;
    })?;
    config.save()?;
    config.reload()?;
    let volume: u8 = config.get("audio.volume")?;
    println!(
        "Saved to {} (volume={volume}):\n{}",
        config.path().display(),
        config.to_toml()
    );
    Ok(())
}
