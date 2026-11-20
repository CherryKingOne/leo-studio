use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::error::CoreResult;

pub type EventHandler = Box<dyn Fn(&serde_json::Value) -> CoreResult<()> + Send + Sync>;

/// 极轻量线程安全事件总线
#[derive(Default, Clone)]
pub struct EventBus {
    listeners: Arc<RwLock<HashMap<String, Vec<Arc<EventHandler>>>>>,
}

impl EventBus {
    pub fn new() -> Self {
        Self {
            listeners: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 订阅指定事件
    pub async fn on<F>(&self, event_name: impl Into<String>, handler: F)
    where
        F: Fn(&serde_json::Value) -> CoreResult<()> + Send + Sync + 'static,
    {
        let mut map = self.listeners.write().await;
        map.entry(event_name.into())
            .or_default()
            .push(Arc::new(Box::new(handler)));
    }

    /// 派发事件
    pub async fn emit(&self, event_name: &str, payload: &serde_json::Value) -> CoreResult<()> {
        let listeners = {
            let map = self.listeners.read().await;
            map.get(event_name).cloned().unwrap_or_default()
        };

        for listener in listeners {
            listener(payload)?;
        }
        Ok(())
    }
}
