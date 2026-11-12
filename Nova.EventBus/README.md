
## 可执行组件模式

`Nova.EventBus` 和 `Nova.Config` 现在都可编译为独立、长期运行的组件进程：

```sh
cargo build --release --bin nova-eventbus
# 产物：Nova.EventBus/target/release/nova-eventbus

cargo build --release --bin nova-config
# 产物：Nova.Config/target/release/nova-config
```

这是“**可执行文件即组件**”模式：主应用或将来的组件宿主负责启动、监控和重启独立程序；组件不需要被链接进主应用。每个组件成功完成初始化后，在标准输出写出一行机器可读就绪记录：

```text
NOVA_COMPONENT_READY {"component":"eventbus","endpoint":"127.0.0.1:47631"}
NOVA_COMPONENT_READY {"component":"config","service":"config"}
```

推荐启动顺序：

```sh
# 终端一：共享中心总线
Nova.EventBus/target/release/nova-eventbus Nova.EventBus/eventbus.toml

# 终端二：配置组件；连接上面的总线并注册 config 服务
Nova.Config/target/release/nova-config Nova.Config/component.toml
```

组件收到 Ctrl+C / SIGINT 后退出；本版本不内置进程拉起、健康检查、重启退避或服务管理，这些属于未来宿主的职责。

### 组件服务路由

EventBus 新增命名服务注册：组件通过 `Client::attach_service("config", service)` 注册，调用方以 `service.method` 形式请求，例如 `config.get`。服务连接断开时，Broker 自动移除其路由；重新连接时 Client 自动重新注册服务和订阅。Broker 仅将调用转发给该服务连接，不执行其业务逻辑。

服务方法必须是 `service.method` 形式；服务名称和方法名都限制为 1–256 个 UTF-8 字节。每个服务名称一次只能由一个已连接组件注册，重复注册失败。该协议升级为版本 2；新 Broker 不能与旧版本 1 客户端互通。

同样适用原有安全限制：仅限回环 TCP，**没有认证、加密或操作系统用户隔离**。不要在不可信本机多用户环境中暴露敏感配置或把监听地址改为非回环地址。
