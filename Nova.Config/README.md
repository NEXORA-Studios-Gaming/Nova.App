
## 可执行 Config 组件

`nova-config` 是独立组件程序，不是一次性命令行编辑器。它启动后连接中心 `nova-eventbus`，注册 `config` 服务，并一直运行到 Ctrl+C / SIGINT。构建并启动：

```sh
cargo build --release --bin nova-config
Nova.Config/target/release/nova-config Nova.Config/component.toml
```

必须先启动 EventBus：

```sh
Nova.EventBus/target/release/nova-eventbus Nova.EventBus/eventbus.toml
Nova.Config/target/release/nova-config Nova.Config/component.toml
```

初始化成功时 Config 会输出：

```text
NOVA_COMPONENT_READY {"component":"config","service":"config"}
```

启动清单是 [`component.toml`](component.toml)。`eventbus_config` 指向共享总线配置，`operation_options` 指向本库的保存策略，`[[configs]]` 声明唯一配置 ID 与允许访问的文件路径。相对路径以清单文件目录为基准。组件只访问清单中登记的文件；配置 ID 只能包含 ASCII 字母、数字、`_` 与 `-`，长度 1–64。

```toml
# component.toml
eventbus_config = "../Nova.EventBus/eventbus.toml"
operation_options = "nova-config.toml"

[[configs]]
id = "game"
path = "game-settings.toml"
```

配置文件缺失时，只有当操作选项中的 `create_if_missing = true` 时才会创建；创建内容为一个空 TOML 表。单个 Config 组件采用单写入者模型，统一持有它登记的多个配置文件。

### 对其他组件的 RPC 接口

调用方同样使用 `nova_eventbus::Client`，请求按 `config.<method>` 发送。所有请求和响应是 MessagePack 编码的 Serde 类型；请求结构体已从 `nova_config` 导出。

| 方法 | 请求 | 成功响应 | 语义 |
| --- | --- | --- | --- |
| `config.list` | `Empty {}` | `Vec<String>` | 返回已登记 ID 的排序列表 |
| `config.get` | `GetRequest { config, key }` | `toml::Value` | 读取一个 TOML dotted-key |
| `config.set` | `SetRequest { config, key, value }` | `bool` (`true`) | 修改、校验并立即保存 |
| `config.remove` | `KeyRequest { config, key }` | `bool` | 删除、校验并立即保存；键不存在时为 `false` |
| `config.reload` | `ConfigRequest { config }` | `bool` (`true`) | 丢弃内存状态，重载并校验磁盘文件 |

每次 `set` / `remove` 成功保存后，组件发布 `config.changed` 事件，类型为 `Changed { config, key }`；`reload` 的 `key` 为 `"*"`。订阅应在写入之前建立，避免错过通知。

```rust,no_run
use nova_config::{Changed, Empty, GetRequest, SetRequest};
use nova_eventbus::{Client, Config, Result};

async fn use_config() -> Result<()> {
    let client = Client::connect(Config::from_file("eventbus.toml")?).await?;
    let ids: Vec<String> = client.request("config.list", &Empty {}).await?;
    let mut changes = client.subscribe::<Changed>("config.changed").await?;

    client.request::<_, bool>("config.set", &SetRequest {
        config: "game".into(),
        key: "ui.theme".into(),
        value: toml::Value::String("dark".into()),
    }).await?;
    assert_eq!(changes.recv().await?.key, "ui.theme");

    let theme: toml::Value = client.request("config.get", &GetRequest {
        config: "game".into(), key: "ui.theme".into(),
    }).await?;
    println!("{ids:?}, {theme}");
    Ok(())
}
```

`examples/component_client.rs` 是可运行的调用示例。远程写入在内存中先执行类型检查和 Config 层保存；保存失败会尝试恢复本次键级修改，并将失败作为远端 RPC 错误返回。调用端若收到 `Timeout` 或 `Disconnected`，写入是否已经落盘仍不确定；应先 `get` 或 `reload` 确认状态，不应盲目重复非幂等操作。

服务断线后 EventBus 自动移除 `config.*` 路由；Config 客户端重连后自动重新注册。由于事件和服务注册均在内存中，离线期间不会保留配置变更通知。
