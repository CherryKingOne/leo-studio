pub mod event;
pub mod service;

use std::sync::{Arc, Mutex, RwLock};

pub use event::EventBus;
pub use service::ServiceContainer;

use crate::error::CoreResult;
use crate::registry::{Origin, Registration, Registry};

type Disposer = Box<dyn FnOnce() + Send + Sync>;

/// Cordis / leo-agents 风格的插件上下文。
///
/// - **服务**：`provide` / `inject`，按类型存放单例服务（如注册层插件对外提供的 `TtsHub`）。
/// - **注册表**：`registry::<dyn X>()` 取某个扩展点的驱动表，`register::<dyn X>(id, impl)` 往里注册驱动。
/// - **事件**：`on` / `emit`。
/// - **插件作用域**：`PluginManager` 给每个插件派生一个带 `owner` 的上下文，插件通过它做的
///   注册 / 订阅 / `on_dispose` 都会被记录，卸载插件或装配失败时按相反顺序自动撤销。
#[derive(Clone)]
pub struct Context {
    services: Arc<RwLock<ServiceContainer>>,
    pub events: EventBus,
    /// 作用域 ID（全局或项目 ID）
    pub scope: String,
    /// 当前插件 id（根上下文为 None）
    owner: Option<String>,
    disposers: Arc<Mutex<Vec<Disposer>>>,
}

impl Default for Context {
    fn default() -> Self {
        Self::new("root")
    }
}

/// 从 `dyn leo_core::spi::tts::TTSProvider` 这样的类型名里取出 `TTSProvider` 作为扩展点名
fn kind_of<T: ?Sized>() -> String {
    let name = std::any::type_name::<T>();
    let name = name.trim_start_matches("dyn ");
    let base = name.split('<').next().unwrap_or(name);
    base.rsplit("::").next().unwrap_or(base).to_string()
}

impl Context {
    pub fn new(scope: impl Into<String>) -> Self {
        Self {
            services: Arc::new(RwLock::new(ServiceContainer::new())),
            events: EventBus::new(),
            scope: scope.into(),
            owner: None,
            disposers: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// 当前插件 id
    pub fn owner(&self) -> Option<&str> {
        self.owner.as_deref()
    }

    /// 为某个插件派生作用域上下文：共享服务、注册表与事件，单独记录撤销函数
    pub fn for_plugin(&self, plugin_id: impl Into<String>) -> Self {
        Self {
            services: self.services.clone(),
            events: self.events.clone(),
            scope: self.scope.clone(),
            owner: Some(plugin_id.into()),
            disposers: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// 派生子上下文（如 Project-Scope），共享服务与撤销列表
    pub fn derive(&self, scope: impl Into<String>) -> Self {
        Self { scope: scope.into(), ..self.clone() }
    }

    /// 登记一个在插件卸载时执行的清理函数
    pub fn on_dispose(&self, f: impl FnOnce() + Send + Sync + 'static) {
        self.disposers.lock().expect("context poisoned").push(Box::new(f));
    }

    /// 按注册的相反顺序执行并清空本作用域的全部清理函数
    pub fn dispose(&self) {
        let list = std::mem::take(&mut *self.disposers.lock().expect("context poisoned"));
        for f in list.into_iter().rev() {
            f();
        }
    }

    // ---------- 服务 ----------

    /// 注册一个服务（插件卸载时自动移除，除非已被后来者覆盖）
    pub async fn provide<T: Send + Sync + 'static>(&self, service: T) {
        let service = Arc::new(service);
        self.services.write().expect("context poisoned").register(service.clone());
        let services = Arc::downgrade(&self.services);
        self.on_dispose(move || {
            if let Some(s) = services.upgrade() {
                s.write().expect("context poisoned").remove_if_same(&service);
            }
        });
    }

    /// 获取一个服务
    pub async fn inject<T: Send + Sync + 'static>(&self) -> CoreResult<Arc<T>> {
        self.services.read().expect("context poisoned").get::<T>()
    }

    pub fn has<T: Send + Sync + 'static>(&self) -> bool {
        self.services.read().expect("context poisoned").has::<T>()
    }

    // ---------- 注册表 ----------

    /// 取某个扩展点的驱动注册表（第一次访问时创建，所以注册层与子插件谁先加载都可以）
    pub fn registry<T: ?Sized + Send + Sync + 'static>(&self) -> Registry<T> {
        let reg = self
            .services
            .write()
            .expect("context poisoned")
            .get_or_insert_with(|| Registry::<T>::new(kind_of::<T>()));
        (*reg).clone()
    }

    /// 以插件身份注册驱动；插件卸载时自动撤销
    pub fn register<T: ?Sized + Send + Sync + 'static>(&self, id: &str, value: Arc<T>) -> CoreResult<()> {
        self.register_with_origin(id, value, Origin::External)
    }

    /// 注册内置驱动（可被同 id 的插件驱动顶替）；一般只有注册层插件自己用
    pub fn register_builtin<T: ?Sized + Send + Sync + 'static>(&self, id: &str, value: Arc<T>) -> CoreResult<()> {
        self.register_with_origin(id, value, Origin::Builtin)
    }

    fn register_with_origin<T: ?Sized + Send + Sync + 'static>(
        &self,
        id: &str,
        value: Arc<T>,
        origin: Origin,
    ) -> CoreResult<()> {
        let registration: Registration = self.registry::<T>().register(id, value, origin, self.owner.clone())?;
        let undo = registration.into_disposer();
        self.on_dispose(undo);
        Ok(())
    }

    // ---------- 事件 ----------

    /// 订阅事件；插件卸载时自动取消
    pub fn on<F>(&self, event: impl Into<String>, handler: F)
    where
        F: Fn(&serde_json::Value) -> CoreResult<()> + Send + Sync + 'static,
    {
        let event = event.into();
        let id = self.events.on(event.clone(), handler);
        let bus = self.events.clone();
        self.on_dispose(move || bus.off(&event, id));
    }

    /// 派发事件
    pub async fn emit(&self, event: &str, payload: &serde_json::Value) -> CoreResult<()> {
        self.events.emit(event, payload)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(dead_code)]
    trait Speak: Send + Sync {
        fn say(&self) -> &'static str;
    }
    struct A;
    impl Speak for A {
        fn say(&self) -> &'static str {
            "a"
        }
    }

    #[test]
    fn kind_name_is_trait_name() {
        assert_eq!(kind_of::<dyn Speak>(), "Speak");
    }

    #[tokio::test]
    async fn plugin_scope_dispose_revokes_everything() {
        let root = Context::new("t");
        let p = root.for_plugin("p");
        p.register::<dyn Speak>("a", Arc::new(A)).unwrap();
        p.provide(42u32).await;
        p.on("evt", |_| Ok(()));
        assert!(root.registry::<dyn Speak>().contains("a"));
        assert_eq!(*root.inject::<u32>().await.unwrap(), 42);
        assert_eq!(root.events.listener_count("evt"), 1);
        p.dispose();
        assert!(!root.registry::<dyn Speak>().contains("a"));
        assert!(root.inject::<u32>().await.is_err());
        assert_eq!(root.events.listener_count("evt"), 0);
    }
}
