use nova_config::{ConfigComponent, Manifest, Options, Result};
use nova_eventbus::{Client, Config, Service};
use std::path::Path;

#[tokio::main]
async fn main() -> Result<()> {
    let manifest_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "component.toml".into());
    let (manifest, base) = Manifest::load(&manifest_path)?;
    let options_path = resolve(&base, &manifest.operation_options);
    let bus_path = resolve(&base, &manifest.eventbus_config);
    let options = Options::from_file(options_path)?;
    let component = ConfigComponent::open(manifest, &base, options)?;
    let client = Client::connect(Config::from_file(bus_path)?)
        .await
        .map_err(|error| nova_config::Error::Validation(format!("EventBus connection: {error}")))?;
    let mut service = Service::new();
    component
        .register(&mut service, client.clone())
        .map_err(|error| nova_config::Error::Validation(format!("Config service: {error}")))?;
    client
        .attach_service("config", service)
        .await
        .map_err(|error| {
            nova_config::Error::Validation(format!("EventBus registration: {error}"))
        })?;
    println!("NOVA_COMPONENT_READY {{\"component\":\"config\",\"service\":\"config\"}}");
    tokio::signal::ctrl_c()
        .await
        .map_err(|error| nova_config::Error::Io {
            path: "signal".into(),
            source: error,
        })?;
    client.close();
    Ok(())
}

fn resolve(base: &Path, path: &Path) -> std::path::PathBuf {
    if path.is_absolute() {
        path.into()
    } else {
        base.join(path)
    }
}
