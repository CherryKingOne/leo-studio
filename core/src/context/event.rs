use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

use crate::error::CoreResult;

pub type EventHandler = Arc<dyn Fn(&serde_json::Value) -> CoreResult<()> + Send + Sync>;

/// 轻量线程安全事件总线。
///
/// 一个监听器失败不会中断其他监听器：全部执行完后返回第一个错误。
#[derive(Default, Clone)]
pub struct EventBus {
    listeners: Arc<RwLock<HashMap<String, Vec<(u64, EventHandler)>>>>,
    next: Arc<AtomicU64>,
}

impl EventBus {
    pub fn new() -> Self {
        Self::default()
    }

    /// 订阅指定事件，返回监听器 id（用于 [`EventBus::off`]）
    pub fn on<F>(&self, event_name: impl Into<String>, handler: F) -> u64
    where
        F: Fn(&serde_json::Value) -> CoreResult<()> + Send + Sync + 'static,
    {
        let id = self.next.fetch_add(1, Ordering::Relaxed) + 1;
        let mut map = self.listeners.write().expect("event bus poisoned");
        map.entry(event_name.into()).or_default().push((id, Arc::new(handler)));
        id
    }

    /// 取消订阅
    pub fn off(&self, event_name: &str, id: u64) {
        let mut map = self.listeners.write().expect("event bus poisoned");
        if let Some(list) = map.get_mut(event_name) {
            list.retain(|(i, _)| *i != id);
            if list.is_empty() {
                map.remove(event_name);
            }
        }
    }

    /// 派发事件
    pub fn emit(&self, event_name: &str, payload: &serde_json::Value) -> CoreResult<()> {
        let listeners: Vec<EventHandler> = {
            let map = self.listeners.read().expect("event bus poisoned");
            map.get(event_name).map(|l| l.iter().map(|(_, h)| h.clone()).collect()).unwrap_or_default()
        };
        let mut first_err = None;
        for listener in listeners {
            if let Err(err) = listener(payload) {
                tracing::warn!("事件 {event_name} 的监听器执行失败：{err}");
                first_err.get_or_insert(err);
            }
        }
        first_err.map_or(Ok(()), Err)
    }

    pub fn listener_count(&self, event_name: &str) -> usize {
        self.listeners.read().expect("event bus poisoned").get(event_name).map_or(0, Vec::len)
    }
}
