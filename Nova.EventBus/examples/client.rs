use nova_eventbus::{Client, Config, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct GameEvent {
    player: String,
    score: u64,
    attachment: Vec<u8>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "eventbus.toml".into());
    let client = Client::connect(Config::from_file(path)?).await?;
    let mut events = client.subscribe::<GameEvent>("game.score").await?;
    client
        .publish(
            "game.score",
            &GameEvent {
                player: "Nova".into(),
                score: 42,
                attachment: vec![0, 1, 128, 255],
            },
        )
        .await?;
    println!("Event: {:?}", events.recv().await?);
    let reply: String = client
        .request("system.echo", &"hello from another process")
        .await?;
    println!("RPC: {reply}");
    client.unsubscribe("game.score").await?;
    client.close();
    Ok(())
}
