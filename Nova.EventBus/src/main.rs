use nova_eventbus::{Broker, Config, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let config_path = args.next().unwrap_or_else(|| "eventbus.toml".into());
    let config = Config::from_file(config_path)?;
    let broker = Broker::bind(config).await?;
    let address = broker.local_addr()?;
    println!("NOVA_COMPONENT_READY {{\"component\":\"eventbus\",\"endpoint\":\"{address}\"}}");
    broker
        .serve(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await
}
