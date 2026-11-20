use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::Arc;

use crate::error::{CoreError, CoreResult};

/// 服务容器：基于 Any 类型抹除的线程安全依赖注入容器。
#[derive(Default, Clone)]
pub struct ServiceContainer {
    services: HashMap<TypeId, Arc<dyn Any + Send + Sync>>,
}

impl ServiceContainer {
    pub fn new() -> Self {
        Self {
            services: HashMap::new(),
        }
    }

    /// 注册一个单例服务
    pub fn register<T: Send + Sync + 'static>(&mut self, service: Arc<T>) {
        self.services.insert(TypeId::of::<T>(), service);
    }

    /// 获取已注册的服务
    pub fn get<T: Send + Sync + 'static>(&self) -> CoreResult<Arc<T>> {
        self.services
            .get(&TypeId::of::<T>())
            .and_then(|any_arc| any_arc.clone().downcast::<T>().ok())
            .ok_or_else(|| {
                CoreError::ServiceNotFound(std::any::type_name::<T>().to_string())
            })
    }

    /// 检查是否存在某服务
    pub fn has<T: 'static>(&self) -> bool {
        self.services.contains_key(&TypeId::of::<T>())
    }
}
