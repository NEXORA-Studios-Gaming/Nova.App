use nova_config::{Changed, Empty, GetRequest, SetRequest};
use nova_eventbus::{Client, Config, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "../Nova.EventBus/eventbus.toml".into());
    let client = Client::connect(Config::from_file(path)?).await?;
    let mut changes = client.subscribe::<Changed>("config.changed").await?;
    let configs: Vec<String> = client.request("config.list", &Empty {}).await?;
    println!("Configs: {configs:?}");
    let value = toml::Value::String("dark".into());
    client
        .request::<_, bool>(
            "config.set",
            &SetRequest {
                config: "game".into(),
                key: "ui.theme".into(),
                value,
            },
        )
        .await?;
    println!("Changed: {:?}", changes.recv().await?);
    let theme: toml::Value = client
        .request(
            "config.get",
            &GetRequest {
                config: "game".into(),
                key: "ui.theme".into(),
            },
        )
        .await?;
    println!("Theme: {theme}");
    client.close();
    Ok(())
}
