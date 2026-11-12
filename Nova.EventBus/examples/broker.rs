use nova_eventbus::{Broker, Config, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "eventbus.toml".into());
    let mut broker = Broker::bind(Config::from_file(path)?).await?;
    broker.handle("system.echo", |text: String| async move {
        Ok(format!("NOVA: {text}"))
    })?;
    println!(
        "Broker listening on {} (Ctrl+C to stop)",
        broker.local_addr()?
    );
    broker
        .serve(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await
}
