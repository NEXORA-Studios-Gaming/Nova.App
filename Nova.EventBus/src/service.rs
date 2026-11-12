use crate::{Error, Result, decode, encode, protocol};
use serde::{Serialize, de::DeserializeOwned};
use std::{collections::HashMap, future::Future, pin::Pin, sync::Arc};

pub(crate) type Handler =
    Arc<dyn Fn(Vec<u8>) -> Pin<Box<dyn Future<Output = Result<Vec<u8>>> + Send>> + Send + Sync>;

/// RPC handlers a component exposes through a named EventBus service.
#[derive(Clone, Default)]
pub struct Service {
    pub(crate) handlers: HashMap<String, Handler>,
}

impl Service {
    pub fn new() -> Self {
        Self::default()
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
    pub(crate) fn methods(&self) -> Vec<String> {
        let mut names: Vec<_> = self.handlers.keys().cloned().collect();
        names.sort();
        names
    }
}
