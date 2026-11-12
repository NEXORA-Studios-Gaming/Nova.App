use nova_config::{
    Changed, ConfigComponent, ConfigEntry, ConfigRequest, Empty, GetRequest, KeyRequest, Manifest,
    Options, SetRequest,
};
use nova_eventbus::{Broker, Client, Config, Service};
use std::{path::PathBuf, time::Duration};
use tokio::sync::oneshot;

async fn start(mut config: Config) -> (Config, oneshot::Sender<()>, tokio::task::JoinHandle<()>) {
    let broker = Broker::bind(config.clone()).await.unwrap();
    config.endpoint = broker.local_addr().unwrap();
    let (stop, shutdown) = oneshot::channel();
    let task = tokio::spawn(async move {
        broker
            .serve(async {
                let _ = shutdown.await;
            })
            .await
            .unwrap();
    });
    (config, stop, task)
}

#[tokio::test]
async fn config_component_routes_typed_rpc_and_persists_changes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.toml");
    let (bus, stop, broker) = start(Config {
        endpoint: "127.0.0.1:0".parse().unwrap(),
        ..Config::default()
    })
    .await;
    let manifest = Manifest {
        eventbus_config: PathBuf::new(),
        operation_options: PathBuf::new(),
        configs: vec![ConfigEntry {
            id: "app".into(),
            path: path.clone(),
        }],
    };
    let component = ConfigComponent::open(
        manifest,
        dir.path(),
        Options {
            create_if_missing: true,
            ..Options::default()
        },
    )
    .unwrap();
    let provider = Client::connect(bus.clone()).await.unwrap();
    let mut service = Service::new();
    component.register(&mut service, provider.clone()).unwrap();
    provider.attach_service("config", service).await.unwrap();

    let caller = Client::connect(bus).await.unwrap();
    let listed: Vec<String> = caller.request("config.list", &Empty {}).await.unwrap();
    assert_eq!(listed, vec!["app"]);
    let mut changes = caller.subscribe::<Changed>("config.changed").await.unwrap();
    assert!(
        caller
            .request::<_, bool>(
                "config.set",
                &SetRequest {
                    config: "app".into(),
                    key: "window.width".into(),
                    value: toml::Value::Integer(1920),
                }
            )
            .await
            .unwrap()
    );
    assert_eq!(changes.recv().await.unwrap().key, "window.width");
    let width: toml::Value = caller
        .request(
            "config.get",
            &GetRequest {
                config: "app".into(),
                key: "window.width".into(),
            },
        )
        .await
        .unwrap();
    assert_eq!(width, toml::Value::Integer(1920));
    assert!(
        std::fs::read_to_string(&path)
            .unwrap()
            .contains("width = 1920")
    );
    assert!(
        caller
            .request::<_, bool>(
                "config.remove",
                &KeyRequest {
                    config: "app".into(),
                    key: "window.width".into()
                }
            )
            .await
            .unwrap()
    );
    assert_eq!(changes.recv().await.unwrap().key, "window.width");
    assert!(
        !caller
            .request::<_, bool>(
                "config.remove",
                &KeyRequest {
                    config: "app".into(),
                    key: "missing".into()
                }
            )
            .await
            .unwrap()
    );
    assert!(
        caller
            .request::<_, bool>(
                "config.reload",
                &ConfigRequest {
                    config: "app".into()
                }
            )
            .await
            .unwrap()
    );
    assert_eq!(changes.recv().await.unwrap().key, "*");
    assert!(matches!(
        caller
            .request::<_, toml::Value>(
                "config.get",
                &GetRequest {
                    config: "missing".into(),
                    key: "x".into()
                }
            )
            .await,
        Err(nova_eventbus::Error::Remote(_))
    ));
    caller.close();
    provider.close();
    stop.send(()).unwrap();
    tokio::time::timeout(Duration::from_secs(2), broker)
        .await
        .unwrap()
        .unwrap();
}
