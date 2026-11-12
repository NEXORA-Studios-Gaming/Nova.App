use crate::{
    Config, Error, Result, decode, encode,
    protocol::{self, Message},
    timed,
};
use serde::{Serialize, de::DeserializeOwned};
use std::{collections::HashMap, marker::PhantomData, sync::Arc, time::Duration};
use tokio::{
    net::TcpStream,
    sync::{broadcast, mpsc, oneshot, watch},
    time::Instant,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Connected,
    Reconnecting,
    Closed,
}

/// Cheap to clone; clones share one connection and reconnect loop.
#[derive(Clone)]
pub struct Client {
    commands: mpsc::Sender<Command>,
    shutdown: watch::Sender<bool>,
    state: watch::Receiver<ConnectionState>,
    config: Arc<Config>,
}

pub struct Subscription<T> {
    events: broadcast::Receiver<Arc<Vec<u8>>>,
    marker: PhantomData<T>,
}

impl<T: DeserializeOwned> Subscription<T> {
    /// Reports local queue overflow instead of silently losing messages.
    pub async fn recv(&mut self) -> Result<T> {
        match self.events.recv().await {
            Ok(bytes) => decode(&bytes),
            Err(broadcast::error::RecvError::Lagged(n)) => Err(Error::Lagged(n)),
            Err(broadcast::error::RecvError::Closed) => Err(Error::Closed),
        }
    }
}

enum Operation {
    Subscribe(String),
    Unsubscribe(String),
    Publish(String, Vec<u8>),
    Request(String, Vec<u8>),
    RegisterService(String, crate::Service),
}
enum Response {
    Data(Vec<u8>),
    Subscription(broadcast::Receiver<Arc<Vec<u8>>>),
    Registered,
}
struct Command {
    operation: Operation,
    reply: oneshot::Sender<Result<Response>>,
    deadline: Instant,
}
struct Pending {
    reply: oneshot::Sender<Result<Response>>,
    subscription: Option<broadcast::Receiver<Arc<Vec<u8>>>>,
    registration: Option<(String, crate::Service)>,
    deadline: Instant,
}
type Topics = HashMap<String, broadcast::Sender<Arc<Vec<u8>>>>;

impl Client {
    /// The initial connection must succeed. Later connection failures trigger automatic reconnect.
    pub async fn connect(config: Config) -> Result<Self> {
        config.validate()?;
        let stream = connect(&config).await?;
        let config = Arc::new(config);
        let (commands, receiver) = mpsc::channel(config.queue_capacity);
        let (shutdown, stop) = watch::channel(false);
        let (status, state) = watch::channel(ConnectionState::Connected);
        tokio::spawn(driver(config.clone(), stream, receiver, stop, status));
        Ok(Self {
            commands,
            shutdown,
            state,
            config,
        })
    }

    pub fn state(&self) -> ConnectionState {
        *self.state.borrow()
    }

    /// Waits up to request_timeout_ms for connection and subscription restoration.
    pub async fn wait_connected(&self) -> Result<()> {
        let mut state = self.state.clone();
        timed(self.config.request_timeout_ms, async move {
            loop {
                let current = *state.borrow_and_update();
                match current {
                    ConnectionState::Connected => return Ok(()),
                    ConnectionState::Closed => return Err(Error::Closed),
                    ConnectionState::Reconnecting => {}
                }
                state.changed().await.map_err(|_| Error::Closed)?;
            }
        })
        .await
    }

    pub async fn subscribe<T: DeserializeOwned>(
        &self,
        topic: impl Into<String>,
    ) -> Result<Subscription<T>> {
        match self.execute(Operation::Subscribe(topic.into())).await? {
            Response::Subscription(events) => Ok(Subscription {
                events,
                marker: PhantomData,
            }),
            _ => Err(Error::Protocol("unexpected subscribe response".into())),
        }
    }

    /// Removes the topic for this shared connection and closes all its local receivers.
    pub async fn unsubscribe(&self, topic: impl Into<String>) -> Result<()> {
        self.execute(Operation::Unsubscribe(topic.into())).await?;
        Ok(())
    }

    /// Completion means the broker accepted the event, not that subscribers processed it.
    pub async fn publish<T: Serialize + ?Sized>(
        &self,
        topic: impl Into<String>,
        value: &T,
    ) -> Result<()> {
        self.execute(Operation::Publish(topic.into(), encode(value)?))
            .await?;
        Ok(())
    }

    pub async fn request<Req: Serialize + ?Sized, Res: DeserializeOwned>(
        &self,
        method: impl Into<String>,
        value: &Req,
    ) -> Result<Res> {
        match self
            .execute(Operation::Request(method.into(), encode(value)?))
            .await?
        {
            Response::Data(bytes) => decode(&bytes),
            _ => Err(Error::Protocol("unexpected RPC response".into())),
        }
    }

    pub async fn attach_service(
        &self,
        name: impl Into<String>,
        service: crate::Service,
    ) -> Result<()> {
        if *self.shutdown.borrow() {
            return Err(Error::Closed);
        }
        let name = name.into();
        protocol::validate_name(&name)?;
        let (reply, receiver) = oneshot::channel();
        let deadline = Instant::now() + Duration::from_millis(self.config.request_timeout_ms);
        timed(self.config.request_timeout_ms, async {
            self.commands
                .send(Command {
                    operation: Operation::RegisterService(name, service),
                    reply,
                    deadline,
                })
                .await
                .map_err(|_| Error::Closed)?;
            receiver.await.map_err(|_| Error::Closed)?
        })
        .await?;
        Ok(())
    }

    /// Closes this connection for all clones. Dropping all clones also stops the driver.
    pub fn close(&self) {
        let _ = self.shutdown.send(true);
    }

    async fn execute(&self, operation: Operation) -> Result<Response> {
        if *self.shutdown.borrow() {
            return Err(Error::Closed);
        }
        let name = match &operation {
            Operation::RegisterService(_, _) => "service-registration",
            Operation::Subscribe(s)
            | Operation::Unsubscribe(s)
            | Operation::Publish(s, _)
            | Operation::Request(s, _) => s,
        };
        protocol::validate_name(name)?;
        let (reply, receiver) = oneshot::channel();
        let deadline = Instant::now() + Duration::from_millis(self.config.request_timeout_ms);
        timed(self.config.request_timeout_ms, async {
            self.commands
                .send(Command {
                    operation,
                    reply,
                    deadline,
                })
                .await
                .map_err(|_| Error::Closed)?;
            receiver.await.map_err(|_| Error::Closed)?
        })
        .await
    }
}

async fn connect(config: &Config) -> Result<TcpStream> {
    timed(config.connect_timeout_ms, async {
        let stream = TcpStream::connect(config.endpoint).await?;
        stream.set_nodelay(true)?;
        Ok(stream)
    })
    .await
}

async fn restore(
    stream: TcpStream,
    config: &Config,
    topics: &Topics,
    services: &HashMap<String, crate::Service>,
) -> Result<(protocol::Writer, protocol::Reader)> {
    let (mut writer, mut reader) = protocol::split(stream, config.max_frame_bytes);
    for topic in topics.keys() {
        timed(
            config.write_timeout_ms,
            protocol::send(
                &mut writer,
                Message::Subscribe {
                    id: 0,
                    topic: topic.clone(),
                },
                config.max_frame_bytes,
            ),
        )
        .await?;
        timed(config.request_timeout_ms, async {
            loop {
                match protocol::receive(&mut reader).await? {
                    Message::Reply { id: 0, result } => {
                        result.map_err(Error::Remote)?;
                        return Ok(());
                    }
                    Message::Event { topic, payload } => deliver(topics, topic, payload),
                    _ => return Err(Error::Protocol("unexpected restoration message".into())),
                }
            }
        })
        .await?;
    }
    for (name, service) in services {
        timed(
            config.write_timeout_ms,
            protocol::send(
                &mut writer,
                Message::RegisterService {
                    id: 0,
                    name: name.clone(),
                    methods: service.methods(),
                },
                config.max_frame_bytes,
            ),
        )
        .await?;
        timed(config.request_timeout_ms, async {
            loop {
                match protocol::receive(&mut reader).await? {
                    Message::Reply { id: 0, result } => {
                        result.map_err(Error::Remote)?;
                        return Ok(());
                    }
                    Message::Event { topic, payload } => deliver(topics, topic, payload),
                    _ => {
                        return Err(Error::Protocol(
                            "unexpected service registration response".into(),
                        ));
                    }
                }
            }
        })
        .await?;
    }
    Ok((writer, reader))
}

fn deliver(topics: &Topics, topic: String, payload: Vec<u8>) {
    if let Some(sender) = topics.get(&topic) {
        let _ = sender.send(Arc::new(payload));
    }
}

async fn driver(
    config: Arc<Config>,
    initial: TcpStream,
    mut commands: mpsc::Receiver<Command>,
    mut stop: watch::Receiver<bool>,
    status: watch::Sender<ConnectionState>,
) {
    let mut topics = Topics::new();
    let mut services = HashMap::<String, crate::Service>::new();
    let (service_results, mut service_result_rx) =
        mpsc::channel::<Message>(config.max_in_flight_requests);
    let mut first = Some(initial);
    let mut next_id = 1u64;
    'driver: loop {
        if *stop.borrow() {
            break;
        }
        let stream = if let Some(stream) = first.take() {
            stream
        } else {
            // During an outage, reject operations instead of buffering stale work for replay.
            let attempt = async {
                tokio::time::sleep(Duration::from_millis(config.reconnect_delay_ms)).await;
                connect(&config).await
            };
            tokio::pin!(attempt);
            loop {
                tokio::select! {
                    _ = stop.changed() => break 'driver,
                    command = commands.recv() => match command {
                        Some(command) => { let _ = command.reply.send(Err(Error::Disconnected)); }
                        None => break 'driver,
                    },
                    result = &mut attempt => match result {
                        Ok(stream) => break stream,
                        Err(_) => continue 'driver,
                    }
                }
            }
        };
        let restored = tokio::select! {
            _ = stop.changed() => break,
            result = restore(stream, &config, &topics, &services) => result,
        };
        let Ok((mut writer, mut reader)) = restored else {
            status.send_replace(ConnectionState::Reconnecting);
            continue;
        };
        status.send_replace(ConnectionState::Connected);
        let mut pending = HashMap::<u64, Pending>::new();
        let mut cleanup = tokio::time::interval(Duration::from_millis(25));
        loop {
            tokio::select! {
                _ = stop.changed() => break,
                _ = cleanup.tick() => {
                    let expired: Vec<_> = pending.iter()
                        .filter(|(_, entry)| entry.reply.is_closed() || entry.deadline <= Instant::now())
                        .map(|(&id, _)| id).collect();
                    for id in expired {
                        if let Some(entry) = pending.remove(&id) { let _ = entry.reply.send(Err(Error::Timeout)); }
                    }
                }
                command = commands.recv() => {
                    let Some(command) = command else { break; };
                    if command.reply.is_closed() || command.deadline <= Instant::now() { continue; }
                    if pending.len() >= config.max_in_flight_requests {
                        let _ = command.reply.send(Err(Error::Remote("client concurrency limit reached".into())));
                        continue;
                    }
                    let id = next_id;
                    next_id = next_id.wrapping_add(1).max(1);
                    let mut subscription = None;
                    let mut registration = None;
                    let message = match command.operation {
                        Operation::RegisterService(name, service) => {
                            let methods = service.methods();
                            registration = Some((name.clone(), service));
                            Message::RegisterService { id, name, methods }
                        }
                        Operation::Subscribe(topic) => {
                            if !topics.contains_key(&topic) && topics.len() >= config.queue_capacity {
                                let _ = command.reply.send(Err(Error::Config("subscription limit reached".into())));
                                continue;
                            }
                            let sender = topics.entry(topic.clone()).or_insert_with(|| broadcast::channel(config.queue_capacity).0);
                            subscription = Some(sender.subscribe());
                            Message::Subscribe { id, topic }
                        }
                        Operation::Unsubscribe(topic) => {
                            topics.remove(&topic);
                            Message::Unsubscribe { id, topic }
                        }
                        Operation::Publish(topic, payload) => Message::Publish { id, topic, payload },
                        Operation::Request(method, payload) => Message::Request { id, method, payload },
                    };
                    let result = timed(config.write_timeout_ms, protocol::send(&mut writer, message, config.max_frame_bytes)).await;
                    if let Err(error) = result {
                        let recoverable = matches!(error, Error::Protocol(_) | Error::Codec(_));
                        let _ = command.reply.send(Err(error));
                        if recoverable { continue; }
                        break;
                    }
                    pending.insert(id, Pending { reply: command.reply, subscription, registration, deadline: command.deadline });
                }
                Some(message) = service_result_rx.recv() => {
                    if timed(config.write_timeout_ms, protocol::send(&mut writer, message, config.max_frame_bytes)).await.is_err() { break; }
                    continue;
                }
                message = protocol::receive(&mut reader) => match message {
                    Ok(Message::Event { topic, payload }) => deliver(&topics, topic, payload),
                    Ok(Message::Invoke { id, method, payload }) => {
                        let handler = services.values().find_map(|service| service.handlers.get(&method)).cloned();
                        let tx = service_results.clone();
                        let timeout = config.request_timeout_ms;
                        tokio::spawn(async move {
                            let result = match handler {
                                Some(handler) => timed(timeout, handler(payload)).await.map_err(|error| error.to_string()),
                                None => Err(format!("unknown service method: {method}")),
                            };
                            let _ = tx.send(Message::ServiceReply { id, result }).await;
                        });
                    }
                    Ok(Message::Reply { id, result }) => {
                        if let Some(entry) = pending.remove(&id) {
                            let registered = entry.registration.is_some();
                            if result.is_ok()
                                && let Some((name, service)) = entry.registration
                            { services.insert(name, service); }
                            let response = result.map(|bytes| match entry.subscription {
                                Some(receiver) => Response::Subscription(receiver),
                                None if registered => Response::Registered,
                                None => Response::Data(bytes),
                            }).map_err(Error::Remote);
                            let _ = entry.reply.send(response);
                        }
                    }
                    _ => break,
                }
            }
        }
        status.send_replace(ConnectionState::Reconnecting);
        for (_, entry) in pending {
            let _ = entry.reply.send(Err(Error::Disconnected));
        }
        if *stop.borrow() || commands.is_closed() {
            break;
        }
    }
    status.send_replace(ConnectionState::Closed);
}
