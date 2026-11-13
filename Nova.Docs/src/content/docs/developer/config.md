---
title: 开发 Config 组件
sidebar:
  order: 3
description: 配置清单、TOML 文件与 Config RPC 的使用方式。
---

Config 是一个独立的长期运行组件。它只访问 `component.toml` 中登记的文件，并以单写入者模型统一管理这些文件。

## 配置组件清单

```toml
eventbus_config = "../Nova.EventBus/eventbus.toml"
operation_options = "nova-config.toml"

[[configs]]
id = "game"
path = "game-settings.toml"
```

相对路径以清单文件所在目录为基准。配置 ID 只能包含 ASCII 字母、数字、`_` 和 `-`，长度为 1–64 个字符。

配置文件不存在时，只有 `operation_options` 中的 `create_if_missing = true` 才会创建文件；初始内容为空 TOML 表。

## RPC 方法

| 方法 | 请求 | 成功响应 |
| --- | --- | --- |
| `config.list` | `Empty {}` | `Vec<String>` |
| `config.get` | `GetRequest { config, key }` | `toml::Value` |
| `config.set` | `SetRequest { config, key, value }` | `bool` |
| `config.remove` | `KeyRequest { config, key }` | `bool` |
| `config.reload` | `ConfigRequest { config }` | `bool` |

`key` 使用 TOML dotted-key，例如 `ui.theme`。`set` 和 `remove` 会先校验内存中的内容，再立即保存到磁盘。

## 变更事件

每次 `set` 或 `remove` 成功保存后，Config 发布 `config.changed` 事件，事件类型为：

```rust
pub struct Changed {
    pub config: String,
    pub key: String,
}
```

`reload` 发布事件时，`key` 为 `"*"`。保存失败会尝试恢复本次键级修改，并将失败作为远端 RPC 错误返回。

## 重连行为

Config 断线后，EventBus 会移除 `config.*` 路由。Config 客户端重连时会重新注册服务和订阅，但离线期间不会补发配置变更通知。
