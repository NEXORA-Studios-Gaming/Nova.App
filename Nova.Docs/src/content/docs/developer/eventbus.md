---
title: 接入 EventBus
sidebar:
  order: 2
description: 从 Rust 组件连接 Nova 消息总线并发送请求、订阅事件。
---

调用方使用 `nova_eventbus::Client` 连接中心总线。连接配置由 `eventbus.toml` 提供，组件间请求和响应使用 Serde 类型进行 MessagePack 编码。

## 建立连接并调用服务

```rust
use nova_eventbus::{Client, Config, Result};
use nova_config::{Empty, GetRequest};

async fn read_theme() -> Result<()> {
    let config = Config::from_file("Nova.EventBus/eventbus.toml")?;
    let client = Client::connect(config).await?;

    let ids: Vec<String> = client.request("config.list", &Empty {}).await?;
    let theme: toml::Value = client
        .request("config.get", &GetRequest {
            config: "game".into(),
            key: "ui.theme".into(),
        })
        .await?;

    println!("{ids:?}: {theme}");
    Ok(())
}
```

方法名采用 `service.method` 格式，例如 `config.get`。服务名称与方法名各自限制为 1–256 个 UTF-8 字节。

## 订阅事件

如果需要观察配置变更，应先订阅再发起写入，避免错过通知：

```rust
use nova_config::{Changed, SetRequest};

let mut changes = client.subscribe::<Changed>("config.changed").await?;
client.request::<_, bool>("config.set", &SetRequest {
    config: "game".into(),
    key: "ui.theme".into(),
    value: toml::Value::String("dark".into()),
}).await?;
let changed = changes.recv().await?;
println!("changed: {}", changed.key);
```

## 错误处理

RPC 超时或断开连接时，调用方无法判断写入是否已经落盘。对于写操作，应先读取或重新加载状态，再决定是否重试，避免盲目重复非幂等操作。订阅通知是内存事件，不会在断线期间缓存。

完整的可运行调用示例位于 `Nova.Config/examples/component_client.rs`。
