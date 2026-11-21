use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::Arc;

use crate::error::{CoreError, CoreResult};

type AnyArc = Arc<dyn Any + Send + Sync>;

/// 服务容器：按类型存放单例服务（同一类型只保留一个，后注册的覆盖先注册的并打印警告）。
///
/// 同一类扩展点需要多个实现时（多个 TTS、多个导出器……）请用 [`crate::registry::Registry`]，不要直接塞进容器。
#[derive(Default, Clone)]
pub struct ServiceContainer {
    services: HashMap<TypeId, AnyArc>,
}

impl ServiceContainer {
    pub fn new() -> Self {
        Self { services: HashMap::new() }
    }

    /// 注册一个单例服务，返回被覆盖的旧服务（如果有）
    pub fn register<T: Send + Sync + 'static>(&mut self, service: Arc<T>) -> Option<AnyArc> {
        let old = self.services.insert(TypeId::of::<T>(), service);
        if old.is_some() {
            tracing::warn!("服务 {} 被重复注册，后注册的实现覆盖了先前的实现", std::any::type_name::<T>());
        }
        old
    }

    /// 获取已注册的服务
    pub fn get<T: Send + Sync + 'static>(&self) -> CoreResult<Arc<T>> {
        self.services
            .get(&TypeId::of::<T>())
            .and_then(|any_arc| any_arc.clone().downcast::<T>().ok())
            .ok_or_else(|| CoreError::ServiceNotFound(std::any::type_name::<T>().to_string()))
    }

    /// 取服务，不存在时用 `init` 创建并登记
    pub fn get_or_insert_with<T: Send + Sync + 'static>(&mut self, init: impl FnOnce() -> T) -> Arc<T> {
        let entry = self.services.entry(TypeId::of::<T>()).or_insert_with(|| Arc::new(init()) as AnyArc);
        entry.clone().downcast::<T>().expect("service type mismatch")
    }

    /// 只有当前登记的正是 `service` 这一份时才移除（防止撤销掉后来覆盖的实现）
    pub fn remove_if_same<T: Send + Sync + 'static>(&mut self, service: &Arc<T>) -> bool {
        let id = TypeId::of::<T>();
        let same = self
            .services
            .get(&id)
            .is_some_and(|cur| std::ptr::addr_eq(Arc::as_ptr(cur), Arc::as_ptr(service)));
        if same {
            self.services.remove(&id);
        }
        same
    }

    /// 检查是否存在某服务
    pub fn has<T: 'static>(&self) -> bool {
        self.services.contains_key(&TypeId::of::<T>())
    }
}
