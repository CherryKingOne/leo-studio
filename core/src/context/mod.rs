pub mod event;
pub mod service;

use std::sync::Arc;
use tokio::sync::RwLock;

pub use event::EventBus;
pub use service::ServiceContainer;

use crate::error::CoreResult;

/// Cordis 风格的插件上下文。
#[derive(Clone)]
pub struct Context {
    pub services: Arc<RwLock<ServiceContainer>>,
    pub events: EventBus,
    /// 作用域 ID（全局或项目 ID）
    pub scope: String,
}

impl Default for Context {
    fn default() -> Self {
        Self::new("root")
    }
}

impl Context {
    pub fn new(scope: impl Into<String>) -> Self {
        Self {
            services: Arc::new(RwLock::new(ServiceContainer::new())),
            events: EventBus::new(),
            scope: scope.into(),
        }
    }

    /// 注册一个服务
    pub async fn provide<T: Send + Sync + 'static>(&self, service: T) {
        let mut services = self.services.write().await;
        services.register(Arc::new(service));
    }

    /// 获取一个服务
    pub async fn inject<T: Send + Sync + 'static>(&self) -> CoreResult<Arc<T>> {
        let services = self.services.read().await;
        services.get::<T>()
    }

    /// 派发事件
    pub async fn emit(&self, event: &str, payload: &serde_json::Value) -> CoreResult<()> {
        self.events.emit(event, payload).await
    }

    /// 派生子上下文（如 Project-Scope）
    pub fn derive(&self, scope: impl Into<String>) -> Self {
        Self {
            services: self.services.clone(),
            events: self.events.clone(),
            scope: scope.into(),
        }
    }
}
