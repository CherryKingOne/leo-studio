//! 通用注册表（注册层的“大箱子”底座）。
//!
//! 参考 leo-plugin 里 gateway 的 `channels/registry.ts` 与 provider 的 `registry.ts`：
//!
//! - 一个扩展点（TTS、存储、导入器……）对应一张 [`Registry<T>`]，表里按 **驱动 id** 存放实现（“小盒子”）。
//! - 每个 id 有两个槽位：**内置**（`builtin`）与 **插件**（`external`）。同一个 id 两者都有时插件那份生效、
//!   内置那份被“顶替”；插件撤销后内置那份自动恢复。注册顺序任意，结果相同。
//! - 注册返回 [`Registration`]，调用 `revoke()` 即撤销；通过插件作用域 `Context` 注册时，插件卸载会自动撤销。
//! - 可以设置**默认驱动**（一般由注册层插件按配置设置）；没设置或默认驱动不存在时，取最早注册的那个。
//!
//! 注册表本身不认识任何具体插件，所以以后新增插件只需要“往表里注册”，不用改内核。

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use serde::{Deserialize, Serialize};

use crate::error::{CoreError, CoreResult};

/// 驱动来源
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Origin {
    /// 注册层自带的内置驱动（可以被同 id 的插件驱动顶替）
    Builtin,
    /// 其他插件注册的驱动
    External,
}

/// 注册表中一个驱动的对外描述（用于列表 / 诊断接口）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DriverInfo {
    pub id: String,
    /// 注册它的插件 id（不经插件作用域注册时为空）
    pub owner: Option<String>,
    pub origin: Origin,
    /// 插件驱动顶替了同 id 的内置驱动
    pub overrides_builtin: bool,
    /// 是否是当前默认驱动
    pub is_default: bool,
}

struct Entry<T: ?Sized> {
    value: Arc<T>,
    owner: Option<String>,
    /// 全局注册序号：决定“最早注册”与列表顺序
    seq: u64,
    /// 每次注册唯一的令牌，防止撤销到后来者
    token: u64,
}

struct Slot<T: ?Sized> {
    builtin: Option<Entry<T>>,
    external: Option<Entry<T>>,
}

impl<T: ?Sized> Default for Slot<T> {
    fn default() -> Self {
        Self { builtin: None, external: None }
    }
}

impl<T: ?Sized> Slot<T> {
    fn active(&self) -> Option<&Entry<T>> {
        self.external.as_ref().or(self.builtin.as_ref())
    }
}

struct Inner<T: ?Sized> {
    slots: HashMap<String, Slot<T>>,
    default_id: Option<String>,
    next: u64,
}

/// 一个扩展点的驱动注册表
pub struct Registry<T: ?Sized + Send + Sync + 'static> {
    /// 扩展点名称（日志 / 报错用），如 `tts`
    kind: String,
    inner: Arc<RwLock<Inner<T>>>,
}

impl<T: ?Sized + Send + Sync + 'static> Clone for Registry<T> {
    fn clone(&self) -> Self {
        Self { kind: self.kind.clone(), inner: self.inner.clone() }
    }
}

/// 一次注册的凭据；`revoke()` 撤销本次注册（重复调用无副作用）
pub struct Registration {
    revoke: Option<Box<dyn FnOnce() + Send + Sync>>,
}

impl Registration {
    pub fn revoke(mut self) {
        if let Some(f) = self.revoke.take() {
            f();
        }
    }

    pub(crate) fn into_disposer(mut self) -> Box<dyn FnOnce() + Send + Sync> {
        self.revoke.take().unwrap_or_else(|| Box::new(|| {}))
    }
}

/// 驱动 id 规则：小写字母 / 数字 / 连字符，字母开头（与 leo-plugin 的 kebab-case 约定一致）
pub fn is_valid_id(id: &str) -> bool {
    let mut chars = id.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_lowercase())
        && id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

impl<T: ?Sized + Send + Sync + 'static> Registry<T> {
    pub fn new(kind: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            inner: Arc::new(RwLock::new(Inner { slots: HashMap::new(), default_id: None, next: 0 })),
        }
    }

    pub fn kind(&self) -> &str {
        &self.kind
    }

    /// 注册驱动。同一 id、同一来源重复注册会报错；插件驱动会顶替同 id 的内置驱动。
    pub fn register(
        &self,
        id: &str,
        value: Arc<T>,
        origin: Origin,
        owner: Option<String>,
    ) -> CoreResult<Registration> {
        if !is_valid_id(id) {
            return Err(CoreError::InvalidParameter(format!(
                "{} 驱动 id \"{id}\" 不合法：只能用小写字母、数字和连字符，且以字母开头",
                self.kind
            )));
        }
        let token = {
            let mut inner = self.inner.write().expect("registry poisoned");
            inner.next += 1;
            let seq = inner.next;
            let slot = inner.slots.entry(id.to_string()).or_default();
            let overrides = origin == Origin::External && slot.builtin.is_some();
            let target = match origin {
                Origin::Builtin => &mut slot.builtin,
                Origin::External => &mut slot.external,
            };
            if let Some(existing) = target {
                return Err(CoreError::Plugin {
                    plugin_id: owner.unwrap_or_default(),
                    message: format!(
                        "{} 驱动 \"{id}\" 已被{}注册",
                        self.kind,
                        existing.owner.as_deref().map(|o| format!("插件 {o} ")).unwrap_or_default()
                    ),
                });
            }
            *target = Some(Entry { value, owner: owner.clone(), seq, token: seq });
            if overrides {
                tracing::info!("{} 驱动 {id} 改用插件提供的实现，内置实现已停用（撤销插件即恢复）", self.kind);
            } else {
                tracing::debug!("{} 驱动 {id} 已注册（{:?}）", self.kind, origin);
            }
            seq
        };

        let inner = Arc::downgrade(&self.inner);
        let kind = self.kind.clone();
        let id = id.to_string();
        Ok(Registration {
            revoke: Some(Box::new(move || {
                let Some(inner) = inner.upgrade() else { return };
                let mut inner = inner.write().expect("registry poisoned");
                let Some(slot) = inner.slots.get_mut(&id) else { return };
                let target = match origin {
                    Origin::Builtin => &mut slot.builtin,
                    Origin::External => &mut slot.external,
                };
                if target.as_ref().map(|e| e.token) != Some(token) {
                    return;
                }
                *target = None;
                let restored = origin == Origin::External && slot.builtin.is_some();
                if slot.builtin.is_none() && slot.external.is_none() {
                    inner.slots.remove(&id);
                }
                if restored {
                    tracing::info!("{kind} 驱动 {id} 的插件实现已撤销，恢复内置实现");
                } else {
                    tracing::debug!("{kind} 驱动 {id} 已撤销");
                }
            })),
        })
    }

    /// 按 id 取当前生效的驱动
    pub fn get(&self, id: &str) -> Option<Arc<T>> {
        let inner = self.inner.read().expect("registry poisoned");
        inner.slots.get(id).and_then(|s| s.active()).map(|e| e.value.clone())
    }

    pub fn contains(&self, id: &str) -> bool {
        self.get(id).is_some()
    }

    /// 设置默认驱动 id（可以先设置后注册；`None` 表示回到“最早注册的那个”）
    pub fn set_default(&self, id: Option<&str>) {
        let mut inner = self.inner.write().expect("registry poisoned");
        inner.default_id = id.map(str::to_string);
    }

    /// 配置的默认驱动 id（不保证已注册）
    pub fn configured_default(&self) -> Option<String> {
        self.inner.read().expect("registry poisoned").default_id.clone()
    }

    /// 当前默认驱动：配置的默认驱动已注册就用它，否则用最早注册的
    pub fn default_entry(&self) -> Option<(String, Arc<T>)> {
        let inner = self.inner.read().expect("registry poisoned");
        if let Some(id) = &inner.default_id {
            if let Some(e) = inner.slots.get(id).and_then(|s| s.active()) {
                return Some((id.clone(), e.value.clone()));
            }
        }
        inner
            .slots
            .iter()
            .filter_map(|(id, s)| s.active().map(|e| (id, e)))
            .min_by_key(|(_, e)| e.seq)
            .map(|(id, e)| (id.clone(), e.value.clone()))
    }

    pub fn default_value(&self) -> Option<Arc<T>> {
        self.default_entry().map(|(_, v)| v)
    }

    /// 取指定 id；`None` 时取默认驱动
    pub fn resolve(&self, id: Option<&str>) -> CoreResult<(String, Arc<T>)> {
        match id {
            Some(id) => self.get(id).map(|v| (id.to_string(), v)).ok_or_else(|| {
                CoreError::ServiceNotFound(format!("{} 驱动 \"{id}\"（已注册：{}）", self.kind, self.ids().join(", ")))
            }),
            None => self
                .default_entry()
                .ok_or_else(|| CoreError::ServiceNotFound(format!("{} 驱动（还没有任何插件注册）", self.kind))),
        }
    }

    /// 全部生效驱动，按注册顺序
    pub fn entries(&self) -> Vec<(String, Arc<T>)> {
        let inner = self.inner.read().expect("registry poisoned");
        let mut list: Vec<_> = inner
            .slots
            .iter()
            .filter_map(|(id, s)| s.active().map(|e| (e.seq, id.clone(), e.value.clone())))
            .collect();
        list.sort_by_key(|(seq, _, _)| *seq);
        list.into_iter().map(|(_, id, v)| (id, v)).collect()
    }

    pub fn ids(&self) -> Vec<String> {
        self.entries().into_iter().map(|(id, _)| id).collect()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.read().expect("registry poisoned").slots.is_empty()
    }

    /// 驱动列表（诊断用）
    pub fn describe(&self) -> Vec<DriverInfo> {
        let default_id = self.default_entry().map(|(id, _)| id);
        let inner = self.inner.read().expect("registry poisoned");
        let mut list: Vec<_> = inner
            .slots
            .iter()
            .filter_map(|(id, s)| {
                let e = s.active()?;
                let origin = if s.external.is_some() { Origin::External } else { Origin::Builtin };
                Some((
                    e.seq,
                    DriverInfo {
                        id: id.clone(),
                        owner: e.owner.clone(),
                        origin,
                        overrides_builtin: s.external.is_some() && s.builtin.is_some(),
                        is_default: default_id.as_deref() == Some(id.as_str()),
                    },
                ))
            })
            .collect();
        list.sort_by_key(|(seq, _)| *seq);
        list.into_iter().map(|(_, i)| i).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    trait Greeter: Send + Sync {
        fn hi(&self) -> String;
    }
    struct G(&'static str);
    impl Greeter for G {
        fn hi(&self) -> String {
            self.0.to_string()
        }
    }

    #[test]
    fn external_overrides_builtin_and_restores() {
        let reg: Registry<dyn Greeter> = Registry::new("greeter");
        let b = reg.register("qq", Arc::new(G("builtin")), Origin::Builtin, None).unwrap();
        let e = reg.register("qq", Arc::new(G("plugin")), Origin::External, Some("p".into())).unwrap();
        assert_eq!(reg.get("qq").unwrap().hi(), "plugin");
        assert!(reg.describe()[0].overrides_builtin);
        e.revoke();
        assert_eq!(reg.get("qq").unwrap().hi(), "builtin");
        b.revoke();
        assert!(reg.get("qq").is_none());
        assert!(reg.is_empty());
    }

    #[test]
    fn duplicate_and_invalid_ids_rejected() {
        let reg: Registry<dyn Greeter> = Registry::new("greeter");
        let _a = reg.register("a", Arc::new(G("1")), Origin::External, None).unwrap();
        assert!(reg.register("a", Arc::new(G("2")), Origin::External, None).is_err());
        assert!(reg.register("Bad_ID", Arc::new(G("2")), Origin::External, None).is_err());
    }

    #[test]
    fn default_falls_back_to_first_registered() {
        let reg: Registry<dyn Greeter> = Registry::new("greeter");
        reg.set_default(Some("b"));
        let _a = reg.register("a", Arc::new(G("a")), Origin::External, None).unwrap();
        assert_eq!(reg.default_entry().unwrap().0, "a");
        let _b = reg.register("b", Arc::new(G("b")), Origin::External, None).unwrap();
        assert_eq!(reg.default_entry().unwrap().0, "b");
        assert_eq!(reg.ids(), vec!["a", "b"]);
    }

    #[test]
    fn stale_revoke_does_not_remove_newer_registration() {
        let reg: Registry<dyn Greeter> = Registry::new("greeter");
        let first = reg.register("a", Arc::new(G("1")), Origin::External, None).unwrap();
        let undo = first.into_disposer();
        undo();
        let _second = reg.register("a", Arc::new(G("2")), Origin::External, None).unwrap();
        assert_eq!(reg.get("a").unwrap().hi(), "2");
    }
}
