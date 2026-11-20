use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::Arc;

use crate::error::{CoreError, CoreResult};

/// 未指定名称时服务登记使用的默认名称。
pub const DEFAULT_SERVICE_NAME: &str = "default";

#[derive(Clone)]
struct ServiceEntry {
    name: String,
    service: Arc<dyn Any + Send + Sync>,
}

/// 同一服务类型下的全部提供者，按登记顺序排列。
#[derive(Default, Clone)]
struct ServiceSlot {
    entries: Vec<ServiceEntry>,
    default: Option<String>,
}

impl ServiceSlot {
    fn find(&self, name: &str) -> Option<&ServiceEntry> {
        self.entries.iter().find(|entry| entry.name == name)
    }

    /// 显式指定的默认提供者优先，否则为最先登记的提供者。
    fn default_entry(&self) -> Option<&ServiceEntry> {
        self.default
            .as_deref()
            .and_then(|name| self.find(name))
            .or_else(|| self.entries.first())
    }
}

/// 服务容器：按服务类型分组、按名称区分多个提供者的线程安全依赖注入容器。
#[derive(Default, Clone)]
pub struct ServiceContainer {
    services: HashMap<TypeId, ServiceSlot>,
}

impl ServiceContainer {
    pub fn new() -> Self {
        Self {
            services: HashMap::new(),
        }
    }

    /// 以默认名称注册一个服务；同类型同名称已存在时返回冲突错误。
    pub fn register<T: Send + Sync + 'static>(&mut self, service: Arc<T>) -> CoreResult<()> {
        self.register_named(DEFAULT_SERVICE_NAME, service)
    }

    /// 以指定名称注册一个服务提供者；同类型同名称已存在时返回冲突错误，不会覆盖。
    pub fn register_named<T: Send + Sync + 'static>(
        &mut self,
        name: impl Into<String>,
        service: Arc<T>,
    ) -> CoreResult<()> {
        let name = name.into();
        let slot = self.services.entry(TypeId::of::<T>()).or_default();
        if slot.find(&name).is_some() {
            return Err(CoreError::ServiceConflict {
                service: std::any::type_name::<T>().to_string(),
                name,
            });
        }
        slot.entries.push(ServiceEntry { name, service });
        Ok(())
    }

    /// 获取默认提供者：显式设置的默认值，否则为最先登记者
    pub fn get<T: Send + Sync + 'static>(&self) -> CoreResult<Arc<T>> {
        self.services
            .get(&TypeId::of::<T>())
            .and_then(ServiceSlot::default_entry)
            .and_then(|entry| entry.service.clone().downcast::<T>().ok())
            .ok_or_else(|| CoreError::ServiceNotFound(std::any::type_name::<T>().to_string()))
    }

    /// 按名称获取服务提供者
    pub fn get_named<T: Send + Sync + 'static>(&self, name: &str) -> CoreResult<Arc<T>> {
        self.services
            .get(&TypeId::of::<T>())
            .and_then(|slot| slot.find(name))
            .and_then(|entry| entry.service.clone().downcast::<T>().ok())
            .ok_or_else(|| {
                CoreError::ServiceNotFound(format!("{}#{name}", std::any::type_name::<T>()))
            })
    }

    /// 按登记顺序返回某服务类型的全部提供者
    pub fn get_all<T: Send + Sync + 'static>(&self) -> Vec<(String, Arc<T>)> {
        self.services
            .get(&TypeId::of::<T>())
            .map(|slot| {
                slot.entries
                    .iter()
                    .filter_map(|entry| {
                        entry
                            .service
                            .clone()
                            .downcast::<T>()
                            .ok()
                            .map(|service| (entry.name.clone(), service))
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// 按登记顺序返回某服务类型的全部提供者名称
    pub fn names<T: 'static>(&self) -> Vec<String> {
        self.services
            .get(&TypeId::of::<T>())
            .map(|slot| slot.entries.iter().map(|entry| entry.name.clone()).collect())
            .unwrap_or_default()
    }

    /// 显式指定某服务类型的默认提供者
    pub fn set_default<T: 'static>(&mut self, name: &str) -> CoreResult<()> {
        let slot = self
            .services
            .get_mut(&TypeId::of::<T>())
            .filter(|slot| slot.find(name).is_some())
            .ok_or_else(|| {
                CoreError::ServiceNotFound(format!("{}#{name}", std::any::type_name::<T>()))
            })?;
        slot.default = Some(name.to_string());
        Ok(())
    }

    /// 移除指定名称的服务提供者，返回是否确有移除
    pub fn remove_named<T: 'static>(&mut self, name: &str) -> bool {
        let Some(slot) = self.services.get_mut(&TypeId::of::<T>()) else {
            return false;
        };
        let before = slot.entries.len();
        slot.entries.retain(|entry| entry.name != name);
        if slot.default.as_deref() == Some(name) {
            slot.default = None;
        }
        before != slot.entries.len()
    }

    /// 检查是否存在某服务
    pub fn has<T: 'static>(&self) -> bool {
        self.services
            .get(&TypeId::of::<T>())
            .is_some_and(|slot| !slot.entries.is_empty())
    }
}
