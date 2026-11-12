use nova_eventbus::{Broker, Client, Config, ConnectionState, Error};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, time::Duration};
use tokio::sync::oneshot;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Payload {
    name: String,
    count: u64,
    data: Vec<u8>,
    tags: BTreeMap<String, Option<i64>>,
    state: State,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
enum State {
    Ready,
    Progress { percent: u8 },
}

fn config() -> Config {
    Config {
        endpoint: "127.0.0.1:0".parse().unwrap(),
        reconnect_delay_ms: 20,
        ..Config::default()
    }
}

async fn start(mut config: Config) -> (Config, oneshot::Sender<()>, tokio::task::JoinHandle<()>) {
    let mut broker = Broker::bind(config.clone()).await.unwrap();
    config.endpoint = broker.local_addr().unwrap();
    broker
        .handle("echo", |value: Payload| async { Ok(value) })
        .unwrap();
    broker
        .handle("ping", |value: String| async move {
            Ok(format!("pong:{value}"))
        })
        .unwrap();
    broker
        .handle("fail", |_: ()| async {
            Err::<(), _>(Error::Remote("business failure".into()))
        })
        .unwrap();
    broker
        .handle("slow", |_: ()| async {
            tokio::time::sleep(Duration::from_secs(2)).await;
            Ok(())
        })
        .unwrap();
    let (stop, shutdown) = oneshot::channel();
    let task = tokio::spawn(async {
        broker
            .serve(async {
                let _ = shutdown.await;
            })
            .await
            .unwrap();
    });
    (config, stop, task)
}

fn sample() -> Payload {
    Payload {
        name: "跨进程 🦀".into(),
        count: u64::MAX,
        data: vec![0, 127, 128, 255],
        tags: BTreeMap::from([("optional".into(), None), ("signed".into(), Some(i64::MIN))]),
        state: State::Progress { percent: 75 },
    }
}

#[tokio::test]
async fn registered_services_route_rpc_and_unregister_on_disconnect() {
    let (config, stop, server) = start(config()).await;
    let provider = Client::connect(config.clone()).await.unwrap();
    let caller = Client::connect(config).await.unwrap();
    let mut service = nova_eventbus::Service::new();
    service
        .handle("read", |value: String| async move {
            Ok(format!("stored:{value}"))
        })
        .unwrap();
    provider.attach_service("settings", service).await.unwrap();
    let result: String = caller.request("settings.read", &"theme").await.unwrap();
    assert_eq!(result, "stored:theme");
    provider.close();
    tokio::time::sleep(Duration::from_millis(20)).await;
    assert!(matches!(
        caller.request::<_, String>("settings.read", &"theme").await,
        Err(Error::Remote(_))
    ));
    caller.close();
    stop.send(()).unwrap();
    server.await.unwrap();
}

#[tokio::test]
async fn typed_pubsub_and_rpc() {
    let (config, stop, server) = start(config()).await;
    let publisher = Client::connect(config.clone()).await.unwrap();
    let subscriber = Client::connect(config).await.unwrap();
    let mut first = subscriber.subscribe::<Payload>("game/state").await.unwrap();
    let mut second = subscriber.subscribe::<Payload>("game/state").await.unwrap();
    let mut other = subscriber.subscribe::<Payload>("other").await.unwrap();
    let value = sample();
    publisher.publish("game/state", &value).await.unwrap();
    assert_eq!(first.recv().await.unwrap(), value);
    assert_eq!(second.recv().await.unwrap(), value);
    assert!(
        tokio::time::timeout(Duration::from_millis(50), other.recv())
            .await
            .is_err()
    );
    let result: Payload = publisher.request("echo", &value).await.unwrap();
    assert_eq!(result, value);
    let error = publisher
        .request::<_, ()>("missing", &())
        .await
        .unwrap_err();
    assert!(matches!(error, Error::Remote(_)));
    let error = publisher.request::<_, ()>("fail", &()).await.unwrap_err();
    assert!(error.to_string().contains("business failure"));
    subscriber.unsubscribe("game/state").await.unwrap();
    assert!(matches!(first.recv().await, Err(Error::Closed)));
    assert!(matches!(second.recv().await, Err(Error::Closed)));
    publisher.close();
    subscriber.close();
    stop.send(()).unwrap();
    server.await.unwrap();
}

#[tokio::test]
async fn reconnect_restores_subscriptions() {
    let (config, stop, server) = start(config()).await;
    let client = Client::connect(config.clone()).await.unwrap();
    let mut subscription = client.subscribe::<String>("restart").await.unwrap();
    stop.send(()).unwrap();
    server.await.unwrap();
    tokio::time::timeout(Duration::from_secs(2), async {
        while client.state() == ConnectionState::Connected {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert!(matches!(
        client.publish("restart", &"offline").await,
        Err(Error::Disconnected)
    ));
    let (_, stop, server) = start(config).await;
    client.wait_connected().await.unwrap();
    client.publish("restart", &"restored").await.unwrap();
    assert_eq!(subscription.recv().await.unwrap(), "restored");
    client.close();
    stop.send(()).unwrap();
    server.await.unwrap();
}

#[tokio::test]
async fn timeout_and_close() {
    let (mut config, stop, server) = start(config()).await;
    config.request_timeout_ms = 60;
    let client = Client::connect(config).await.unwrap();
    assert!(matches!(
        client.request::<_, ()>("slow", &()).await,
        Err(Error::Timeout)
    ));
    // A slow RPC does not block unrelated requests.
    let result: String = client.request("ping", &"quick").await.unwrap();
    assert_eq!(result, "pong:quick");
    client.close();
    assert!(matches!(client.publish("x", &()).await, Err(Error::Closed)));
    stop.send(()).unwrap();
    server.await.unwrap();
}

#[tokio::test]
async fn bounded_subscription_reports_lag() {
    let (mut config, stop, server) = start(config()).await;
    config.queue_capacity = 2;
    let client = Client::connect(config).await.unwrap();
    let mut subscription = client.subscribe::<u32>("numbers").await.unwrap();
    for value in 0..10u32 {
        client.publish("numbers", &value).await.unwrap();
    }
    // A round trip guarantees earlier event frames have passed through the client reader.
    let _: String = client.request("ping", &"barrier").await.unwrap();
    assert!(matches!(subscription.recv().await, Err(Error::Lagged(_))));
    client.close();
    stop.send(()).unwrap();
    server.await.unwrap();
}

#[tokio::test]
async fn oversized_frame_does_not_break_connection() {
    let (mut config, stop, server) = start(config()).await;
    config.max_frame_bytes = 256;
    let client = Client::connect(config).await.unwrap();
    assert!(matches!(
        client.publish("big", &vec![0u8; 1024]).await,
        Err(Error::Protocol(_))
    ));
    let result: String = client.request("ping", &"alive").await.unwrap();
    assert_eq!(result, "pong:alive");
    client.close();
    stop.send(()).unwrap();
    server.await.unwrap();
}

#[tokio::test]
async fn malformed_peer_is_disconnected_without_harming_broker() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let (config, stop, server) = start(config()).await;
    for frame in [vec![0xff, 0xff, 0xff, 0xff], vec![0, 0, 0, 1, 0xc1]] {
        let mut stream = tokio::net::TcpStream::connect(config.endpoint)
            .await
            .unwrap();
        stream.write_all(&frame).await.unwrap();
        let mut byte = [0u8];
        let read = tokio::time::timeout(Duration::from_secs(2), stream.read(&mut byte))
            .await
            .unwrap();
        assert!(matches!(read, Ok(0) | Err(_)));
    }
    let client = Client::connect(config).await.unwrap();
    let reply: String = client.request("ping", &"healthy").await.unwrap();
    assert_eq!(reply, "pong:healthy");
    client.close();
    stop.send(()).unwrap();
    server.await.unwrap();
}

#[tokio::test]
async fn interrupted_request_is_not_replayed() {
    let mut settings = config();
    let mut broker = Broker::bind(settings.clone()).await.unwrap();
    settings.endpoint = broker.local_addr().unwrap();
    let (started, mut received) = tokio::sync::mpsc::channel(1);
    broker
        .handle("inflight", move |_: ()| {
            let started = started.clone();
            async move {
                started.send(()).await.unwrap();
                std::future::pending::<nova_eventbus::Result<()>>().await
            }
        })
        .unwrap();
    let (stop, shutdown) = oneshot::channel();
    let task = tokio::spawn(broker.serve(async {
        let _ = shutdown.await;
    }));
    let client = Client::connect(settings.clone()).await.unwrap();
    let requester = client.clone();
    let request = tokio::spawn(async move { requester.request::<_, ()>("inflight", &()).await });
    tokio::time::timeout(Duration::from_secs(2), received.recv())
        .await
        .unwrap()
        .unwrap();
    stop.send(()).unwrap();
    task.await.unwrap().unwrap();
    assert!(matches!(request.await.unwrap(), Err(Error::Disconnected)));

    let mut broker = Broker::bind(settings).await.unwrap();
    let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let observed = calls.clone();
    broker
        .handle("inflight", move |_: ()| {
            observed.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            async { Ok(()) }
        })
        .unwrap();
    broker.handle("barrier", |_: ()| async { Ok(()) }).unwrap();
    let (stop, shutdown) = oneshot::channel();
    let task = tokio::spawn(broker.serve(async {
        let _ = shutdown.await;
    }));
    client.wait_connected().await.unwrap();
    client.request::<_, ()>("barrier", &()).await.unwrap();
    assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 0);
    client.close();
    stop.send(()).unwrap();
    task.await.unwrap().unwrap();
}

#[test]
fn configuration_validation() {
    Config::from_file(concat!(env!("CARGO_MANIFEST_DIR"), "/eventbus.toml")).unwrap();
    let mut settings = config();
    settings.endpoint = "0.0.0.0:12345".parse().unwrap();
    assert!(settings.validate().is_err());
    settings = config();
    settings.queue_capacity = 0;
    assert!(settings.validate().is_err());
    settings = config();
    settings.max_frame_bytes = usize::MAX;
    assert!(settings.validate().is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn independent_process_communicates() {
    let (config, stop, server) = start(config()).await;
    let endpoint = config.endpoint.to_string();
    let output = tokio::task::spawn_blocking(move || {
        std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "subprocess_peer", "--ignored", "--nocapture"])
            .env("NOVA_TEST_ENDPOINT", endpoint)
            .output()
            .unwrap()
    })
    .await
    .unwrap();
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    stop.send(()).unwrap();
    server.await.unwrap();
}

#[test]
#[ignore = "launched by independent_process_communicates"]
fn subprocess_peer() {
    let endpoint = std::env::var("NOVA_TEST_ENDPOINT").expect("must be launched by parent test");
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let settings = Config {
                endpoint: endpoint.parse().unwrap(),
                ..Config::default()
            };
            let client = Client::connect(settings).await.unwrap();
            let value = sample();
            let result: Payload = client.request("echo", &value).await.unwrap();
            assert_eq!(result, value);
            let mut sub = client.subscribe::<Payload>("process/event").await.unwrap();
            client.publish("process/event", &value).await.unwrap();
            assert_eq!(
                tokio::time::timeout(Duration::from_secs(3), sub.recv())
                    .await
                    .unwrap()
                    .unwrap(),
                value
            );
            client.close();
        });
}
