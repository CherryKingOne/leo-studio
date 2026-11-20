pub mod event;
pub mod service;

use std::sync::Arc;
use tokio::sync::RwLock;

pub use event::EventBus;
pub use service::{DEFAULT_SERVICE_NAME, ServiceContainer};

use crate::error::CoreResult;

/// Cordis 风格的插件上下文。
#[derive(Clone)]
pub struct Context {
    pub services: Arc<RwLock<ServiceContainer>>,
    pub events: EventBus,
    /// 作用域 ID（全局或项目 ID）
    pub scope: String,
    /// 当前正在挂载的插件 ID，用于记录服务归属
    owner: Option<String>,
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
            owner: None,
        }
    }

    /// 返回以指定插件身份注册服务的上下文视图，共享同一服务容器与事件总线
    pub fn for_plugin(&self, plugin_id: &str) -> Self {
        Self {
            owner: Some(plugin_id.to_string()),
            ..self.clone()
        }
    }

    /// 以默认名称注册一个服务；重复注册返回冲突错误
    pub async fn provide<T: Send + Sync + 'static>(&self, service: T) -> CoreResult<()> {
        self.provide_named(DEFAULT_SERVICE_NAME, service).await
    }

    /// 以指定名称注册一个服务提供者，允许同类型多个提供者并存
    pub async fn provide_named<T: Send + Sync + 'static>(
        &self,
        name: impl Into<String>,
        service: T,
    ) -> CoreResult<()> {
        let mut services = self.services.write().await;
        services.register_owned(name, Arc::new(service), self.owner.clone())
    }

    /// 获取一个服务的默认提供者（显式默认值，否则为最先登记者）
    pub async fn inject<T: Send + Sync + 'static>(&self) -> CoreResult<Arc<T>> {
        let services = self.services.read().await;
        services.get::<T>()
    }

    /// 按名称获取服务提供者
    pub async fn inject_named<T: Send + Sync + 'static>(&self, name: &str) -> CoreResult<Arc<T>> {
        let services = self.services.read().await;
        services.get_named::<T>(name)
    }

    /// 按登记顺序获取某服务类型的全部提供者
    pub async fn inject_all<T: Send + Sync + 'static>(&self) -> Vec<(String, Arc<T>)> {
        let services = self.services.read().await;
        services.get_all::<T>()
    }

    /// 显式指定某服务类型的默认提供者
    pub async fn set_default<T: Send + Sync + 'static>(&self, name: &str) -> CoreResult<()> {
        let mut services = self.services.write().await;
        services.set_default::<T>(name)
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
            owner: self.owner.clone(),
        }
    }
}
