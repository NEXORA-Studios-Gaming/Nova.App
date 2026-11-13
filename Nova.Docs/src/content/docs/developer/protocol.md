---
title: EventBus 与 Config API
sidebar:
  order: 4
description: 服务路由、RPC、事件和协议约束的快速参考。
---

## 服务路由

服务调用使用 `service.method` 格式。当前可用的 Config 方法如下：

| 方法 | 请求 | 成功响应 |
| --- | --- | --- |
| `config.list` | `Empty {}` | `Vec<String>` |
| `config.get` | `GetRequest { config, key }` | `toml::Value` |
| `config.set` | `SetRequest { config, key, value }` | `bool` |
| `config.remove` | `KeyRequest { config, key }` | `bool` |
| `config.reload` | `ConfigRequest { config }` | `bool` |

服务名称和方法名限制为 1–256 个 UTF-8 字节。每个服务名称一次只能由一个已连接组件注册。

## 配置事件

写入操作成功保存后，Config 发布 `config.changed`：

```rust
pub struct Changed {
    pub config: String,
    pub key: String,
}
```

`set` 和 `remove` 使用具体的键名；`reload` 使用 `"*"` 表示整个配置重新载入。

## 传输与版本

请求和响应使用 Serde 类型进行 MessagePack 编码。当前协议版本为 2，新版 Broker 不与协议版本 1 的客户端互通。

EventBus 默认只接受回环 TCP 连接，当前没有认证、加密或操作系统用户隔离。生产环境或不可信多用户环境中不得直接暴露监听地址。

## 连接错误

客户端遇到 `Timeout` 或 `Disconnected` 时，写入是否已经落盘是不确定的。对非幂等操作，先用 `get` 或 `reload` 确认状态，再决定是否重试。EventBus 不会缓存断线期间的事件。
