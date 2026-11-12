use crate::{
    Config, Error, Result, decode, encode,
    protocol::{self, Message},
    service::Handler,
    timed,
};
use serde::{Serialize, de::DeserializeOwned};
use std::{
    collections::{HashMap, HashSet},
    future::Future,
    net::SocketAddr,
    sync::{
        Arc, Mutex, RwLock,
        atomic::{AtomicU64, AtomicUsize, Ordering},
    },
};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{Semaphore, broadcast, mpsc},
    task::JoinSet,
};

type Services = Arc<RwLock<HashMap<String, ServiceTarget>>>;
type Connections = Arc<RwLock<HashMap<usize, mpsc::Sender<Message>>>>;
type PendingCalls = Arc<Mutex<HashMap<u64, (usize, u64)>>>;

#[derive(Clone)]
struct ServiceTarget {
    owner: usize,
    methods: HashSet<String>,
    outbound: mpsc::Sender<Message>,
}

static NEXT_CONNECTION: AtomicUsize = AtomicUsize::new(1);
static NEXT_INVOCATION: AtomicU64 = AtomicU64::new(1);

/// The local EventBus broker. Registered component services are addressed as `service.method`.
pub struct Broker {
    listener: TcpListener,
    config: Config,
    handlers: HashMap<String, Handler>,
}

impl Broker {
    pub async fn bind(config: Config) -> Result<Self> {
        config.validate()?;
        let listener = TcpListener::bind(config.endpoint).await?;
        Ok(Self {
            listener,
            config,
            handlers: HashMap::new(),
        })
    }
    pub fn local_addr(&self) -> Result<SocketAddr> {
        Ok(self.listener.local_addr()?)
    }
    pub fn handle<Req, Res, F, Fut>(&mut self, method: impl Into<String>, handler: F) -> Result<()>
    where
        Req: DeserializeOwned + Send + 'static,
        Res: Serialize + Send + 'static,
        F: Fn(Req) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Res>> + Send + 'static,
    {
        let method = method.into();
        protocol::validate_name(&method)?;
        if self.handlers.contains_key(&method) {
            return Err(Error::Config(format!("duplicate RPC handler: {method}")));
        }
        let handler = Arc::new(handler);
        self.handlers.insert(
            method,
            Arc::new(move |bytes| {
                let handler = handler.clone();
                Box::pin(async move { encode(&handler(decode(&bytes)?).await?) })
            }),
        );
        Ok(())
    }
    pub async fn serve(self, shutdown: impl Future<Output = ()>) -> Result<()> {
        let (events, _) = broadcast::channel(self.config.queue_capacity);
        let handlers = Arc::new(self.handlers);
        let services: Services = Arc::new(RwLock::new(HashMap::new()));
        let connections: Connections = Arc::new(RwLock::new(HashMap::new()));
        let pending: PendingCalls = Arc::new(Mutex::new(HashMap::new()));
        let limits = Arc::new(Semaphore::new(self.config.max_connections));
        let mut tasks = JoinSet::new();
        tokio::pin!(shutdown);
        loop {
            tokio::select! {
                _ = &mut shutdown => break,
                Some(_) = tasks.join_next(), if !tasks.is_empty() => {},
                accepted = self.listener.accept() => {
                    let (stream, _) = accepted?;
                    let Ok(permit) = limits.clone().try_acquire_owned() else { continue };
                    let id = NEXT_CONNECTION.fetch_add(1, Ordering::Relaxed);
                    let (outbound, receiver) = mpsc::channel(self.config.queue_capacity);
                    connections.write().map_err(|_| Error::Protocol("connection registry unavailable".into()))?.insert(id, outbound);
                    let context = Context { services: services.clone(), connections: connections.clone(), pending: pending.clone() };
                    let receiver_events = events.subscribe(); let event_sender = events.clone(); let config = self.config.clone(); let handlers = handlers.clone();
                    tasks.spawn(async move { let _permit = permit; let _ = connection(stream, config, handlers, event_sender, receiver_events, receiver, context, id).await; });
                }
            }
        }
        tasks.abort_all();
        while tasks.join_next().await.is_some() {}
        Ok(())
    }
}

#[derive(Clone)]
struct Event {
    topic: String,
    payload: Vec<u8>,
}
#[derive(Clone)]
struct Context {
    services: Services,
    connections: Connections,
    pending: PendingCalls,
}
struct Cleanup {
    context: Context,
    id: usize,
    names: Mutex<HashSet<String>>,
}
impl Drop for Cleanup {
    fn drop(&mut self) {
        if let Ok(mut services) = self.context.services.write() {
            services.retain(|_, target| target.owner != self.id);
        }
        if let Ok(mut connections) = self.context.connections.write() {
            connections.remove(&self.id);
        }
        if let Ok(mut pending) = self.context.pending.lock() {
            pending.retain(|_, (caller, _)| *caller != self.id);
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn connection(
    stream: TcpStream,
    config: Config,
    handlers: Arc<HashMap<String, Handler>>,
    events: broadcast::Sender<Event>,
    mut events_rx: broadcast::Receiver<Event>,
    mut outbound: mpsc::Receiver<Message>,
    context: Context,
    id: usize,
) -> Result<()> {
    let cleanup = Cleanup {
        context: context.clone(),
        id,
        names: Mutex::new(HashSet::new()),
    };
    stream.set_nodelay(true)?;
    let (mut writer, mut reader) = protocol::split(stream, config.max_frame_bytes);
    let mut topics = HashSet::new();
    let (results_tx, mut results_rx) = mpsc::channel(config.max_in_flight_requests);
    let mut work = JoinSet::new();
    loop {
        let outgoing = tokio::select! {
            Some(_) = work.join_next(), if !work.is_empty() => continue,
            Some(message) = outbound.recv() => message,
            Some(message) = results_rx.recv() => message,
            event = events_rx.recv() => { let event = event.map_err(|_| Error::Disconnected)?; if !topics.contains(&event.topic) { continue; } Message::Event { topic: event.topic, payload: event.payload } },
            incoming = protocol::receive(&mut reader) => match incoming? {
                Message::Subscribe { id: request, topic } => { protocol::validate_name(&topic)?; let result = if topics.contains(&topic) || topics.len() < config.queue_capacity { topics.insert(topic); Ok(Vec::new()) } else { Err("subscription limit reached".into()) }; Message::Reply { id: request, result } }
                Message::Unsubscribe { id: request, topic } => { protocol::validate_name(&topic)?; topics.remove(&topic); Message::Reply { id: request, result: Ok(Vec::new()) } }
                Message::Publish { id: request, topic, payload } => { protocol::validate_name(&topic)?; let _ = events.send(Event { topic, payload }); Message::Reply { id: request, result: Ok(Vec::new()) } }
                Message::RegisterService { id: request, name, methods } => {
                    protocol::validate_name(&name)?; let methods: Result<HashSet<_>> = methods.into_iter().map(|method| { protocol::validate_name(&method)?; Ok(method) }).collect();
                    let result = match methods {
                        Err(error) => Err(error.to_string()), Ok(methods) if methods.is_empty() => Err("service must provide at least one method".into()), Ok(methods) => {
                            let target = ServiceTarget { owner: id, methods, outbound: context.connections.read().ok().and_then(|c| c.get(&id).cloned()).ok_or(Error::Disconnected)? };
                            let mut services = context.services.write().map_err(|_| Error::Protocol("service registry unavailable".into()))?;
                            if services.contains_key(&name) { Err(format!("service already registered: {name}")) } else { services.insert(name.clone(), target); cleanup.names.lock().map_err(|_| Error::Protocol("service cleanup unavailable".into()))?.insert(name); Ok(Vec::new()) }
                        }
                    }; Message::Reply { id: request, result }
                }
                Message::Request { id: request, method, payload } => {
                    protocol::validate_name(&method)?;
                    if let Some(handler) = handlers.get(&method).cloned() {
                        if work.len() >= config.max_in_flight_requests { Message::Reply { id: request, result: Err("RPC concurrency limit reached".into()) } } else { let tx = results_tx.clone(); let timeout = config.request_timeout_ms; work.spawn(async move { let result = timed(timeout, handler(payload)).await.map_err(|e| e.to_string()); let _ = tx.send(Message::Reply { id: request, result }).await; }); continue; }
                    } else if let Some((service, member)) = method.split_once('.') {
                        let target = context.services.read().ok().and_then(|services| services.get(service).cloned());
                        match target {
                            Some(target) if target.methods.contains(member) => {
                                let invoke = NEXT_INVOCATION.fetch_add(1, Ordering::Relaxed);
                                context.pending.lock().map_err(|_| Error::Protocol("pending-call registry unavailable".into()))?.insert(invoke, (id, request));
                                if target.outbound.send(Message::Invoke { id: invoke, method: member.into(), payload }).await.is_err() { context.pending.lock().ok().and_then(|mut pending| pending.remove(&invoke)); Message::Reply { id: request, result: Err(format!("service unavailable: {service}")) } } else { continue }
                            }
                            _ => Message::Reply { id: request, result: Err(format!("unknown RPC method: {method}")) },
                        }
                    } else { Message::Reply { id: request, result: Err(format!("unknown RPC method: {method}")) } }
                }
                Message::ServiceReply { id: invoke, result } => {
                    let target = context.pending.lock().map_err(|_| Error::Protocol("pending-call registry unavailable".into()))?.remove(&invoke);
                    if let Some((caller, request)) = target
                        && let Some(sender) = context.connections.read().ok().and_then(|connections| connections.get(&caller).cloned())
                    { let _ = sender.send(Message::Reply { id: request, result }).await; }
                    continue;
                }
                _ => return Err(Error::Protocol("unexpected client message".into())),
            }
        };
        timed(
            config.write_timeout_ms,
            protocol::send(&mut writer, outgoing, config.max_frame_bytes),
        )
        .await?;
    }
}
