---
title: 组件架构
sidebar:
  order: 1
description: 了解 Nova 独立组件、消息总线与配置服务的协作方式。
---

Nova 的后端功能按独立进程组织。当前仓库中，EventBus 提供进程间通信，Config 作为一个长期运行的组件连接总线并注册 `config` 服务。

## 进程关系

```text
调用方组件 ── RPC / 事件 ──> EventBus <── 注册服务 ── Config
```

主应用或未来的组件宿主负责启动、监控和重启各个可执行组件。当前组件本身不会管理其他进程，也没有内置健康检查或重启退避。

## 生命周期

1. 启动 `nova-eventbus`，读取 `eventbus.toml` 并绑定回环 TCP 端点。
2. 启动 `nova-config`，读取 `component.toml`，连接总线并注册 `config` 服务。
3. 组件完成初始化后输出一行 `NOVA_COMPONENT_READY` 机器可读记录。
4. 收到 Ctrl+C / SIGINT 后，组件正常退出。

服务注册和事件订阅保存在内存中。客户端断线时 Broker 会移除对应路由；客户端重连后会重新注册服务和订阅。离线期间发布的事件不会保留。

## 工程边界

每个组件可独立构建，不需要链接进 Launcher。组件使用 `nova_eventbus::Client` 通信；业务请求通过命名服务路由转发。Broker 负责连接和路由，不执行 Config 等服务的业务逻辑。

当前协议限制为版本 2，不能与协议版本 1 的客户端互通。通信仅限回环 TCP，未提供认证、加密或用户隔离。
