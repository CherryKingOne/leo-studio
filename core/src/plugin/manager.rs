use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use serde::Serialize;
use tokio::sync::Mutex;

use crate::context::Context;
use crate::error::{CoreError, CoreResult};
use crate::plugin::manifest::PluginManifest;
use crate::plugin::traits::Plugin;

/// 插件状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginState {
    /// 已登记，等待装配
    Pending,
    /// 已装配
    Active,
    /// 装配失败（已回滚它做过的注册）
    Failed,
}

/// 插件状态快照（诊断接口用）
#[derive(Debug, Clone, Serialize)]
pub struct PluginStatus {
    #[serde(flatten)]
    pub manifest: PluginManifest,
    pub state: PluginState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

struct Slot {
    plugin: Arc<dyn Plugin>,
    manifest: PluginManifest,
    state: PluginState,
    error: Option<String>,
    scope: Option<Context>,
}

/// 插件管理器：登记、按依赖顺序装配、失败回滚、卸载。
///
/// - 同 id 重复登记会报错（不再静默覆盖）。
/// - 按登记顺序装配；`manifest.depends` 里声明的插件会先装配。
/// - 每个插件拿到自己的作用域 `Context`，卸载或装配失败时它做过的注册全部撤销。
/// - `apply_all` 可以多次调用，只装配还没装配的插件，所以后登记的插件也会被装配。
#[derive(Default, Clone)]
pub struct PluginManager {
    slots: Arc<Mutex<Vec<Slot>>>,
}

impl PluginManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// 登记一个插件（不装配）
    pub async fn register(&self, plugin: Arc<dyn Plugin>) -> CoreResult<()> {
        let manifest = plugin.manifest();
        let mut slots = self.slots.lock().await;
        if slots.iter().any(|s| s.manifest.id == manifest.id) {
            return Err(CoreError::Plugin {
                plugin_id: manifest.id.clone(),
                message: "同 id 的插件已经登记过".into(),
            });
        }
        slots.push(Slot { plugin, manifest, state: PluginState::Pending, error: None, scope: None });
        Ok(())
    }

    /// 获取所有已登记插件的清单列表（登记顺序）
    pub async fn list_manifests(&self) -> Vec<PluginManifest> {
        self.slots.lock().await.iter().map(|s| s.manifest.clone()).collect()
    }

    /// 插件状态列表
    pub async fn statuses(&self) -> Vec<PluginStatus> {
        self.slots
            .lock()
            .await
            .iter()
            .map(|s| PluginStatus { manifest: s.manifest.clone(), state: s.state, error: s.error.clone() })
            .collect()
    }

    pub async fn state_of(&self, id: &str) -> Option<PluginState> {
        self.slots.lock().await.iter().find(|s| s.manifest.id == id).map(|s| s.state)
    }

    /// 计算装配顺序：登记顺序为基础，依赖先行；缺依赖或循环依赖报错
    fn order(slots: &[Slot]) -> CoreResult<Vec<usize>> {
        let index: HashMap<&str, usize> = slots.iter().enumerate().map(|(i, s)| (s.manifest.id.as_str(), i)).collect();
        let mut done = HashSet::new();
        let mut visiting = HashSet::new();
        let mut out = Vec::new();

        fn visit(
            i: usize,
            slots: &[Slot],
            index: &HashMap<&str, usize>,
            done: &mut HashSet<usize>,
            visiting: &mut HashSet<usize>,
            out: &mut Vec<usize>,
        ) -> CoreResult<()> {
            if done.contains(&i) {
                return Ok(());
            }
            let id = &slots[i].manifest.id;
            if !visiting.insert(i) {
                return Err(CoreError::Plugin { plugin_id: id.clone(), message: "插件之间存在循环依赖".into() });
            }
            for dep in &slots[i].manifest.depends {
                let Some(&j) = index.get(dep.as_str()) else {
                    return Err(CoreError::Plugin { plugin_id: id.clone(), message: format!("依赖的插件 {dep} 没有登记") });
                };
                visit(j, slots, index, done, visiting, out)?;
            }
            visiting.remove(&i);
            done.insert(i);
            out.push(i);
            Ok(())
        }

        for i in 0..slots.len() {
            visit(i, slots, &index, &mut done, &mut visiting, &mut out)?;
        }
        Ok(out)
    }

    /// 装配所有尚未装配的插件。任何一个失败：本次装配的插件全部按相反顺序卸载，返回错误。
    pub async fn apply_all(&self, ctx: &Context) -> CoreResult<()> {
        let mut slots = self.slots.lock().await;
        let order = Self::order(&slots)?;
        let mut applied_now = Vec::new();

        for i in order {
            if slots[i].state == PluginState::Active {
                continue;
            }
            let scope = ctx.for_plugin(slots[i].manifest.id.clone());
            let plugin = slots[i].plugin.clone();
            tracing::info!("正在装配插件: {} ({})", slots[i].manifest.name, slots[i].manifest.id);
            match plugin.apply(&scope).await {
                Ok(()) => {
                    slots[i].state = PluginState::Active;
                    slots[i].error = None;
                    slots[i].scope = Some(scope);
                    applied_now.push(i);
                }
                Err(err) => {
                    scope.dispose();
                    slots[i].state = PluginState::Failed;
                    slots[i].error = Some(err.to_string());
                    for &j in applied_now.iter().rev() {
                        if let Some(s) = slots[j].scope.take() {
                            s.dispose();
                        }
                        slots[j].state = PluginState::Pending;
                    }
                    let id = slots[i].manifest.id.clone();
                    tracing::error!("插件 {id} 装配失败，已回滚本次装配的 {} 个插件：{err}", applied_now.len());
                    return Err(CoreError::Plugin { plugin_id: id, message: err.to_string() });
                }
            }
        }
        Ok(())
    }

    /// 卸载插件：撤销它的全部注册并移出管理器。依赖它的已装配插件会先被卸载（回到 Pending）。
    pub async fn unload(&self, id: &str) -> CoreResult<()> {
        let mut slots = self.slots.lock().await;
        let Some(pos) = slots.iter().position(|s| s.manifest.id == id) else {
            return Err(CoreError::Plugin { plugin_id: id.into(), message: "插件不存在".into() });
        };
        let mut to_reset = vec![id.to_string()];
        let mut k = 0;
        while k < to_reset.len() {
            let cur = to_reset[k].clone();
            for s in slots.iter() {
                if s.manifest.depends.iter().any(|d| *d == cur) && !to_reset.contains(&s.manifest.id) {
                    to_reset.push(s.manifest.id.clone());
                }
            }
            k += 1;
        }
        for dep_id in to_reset.iter().skip(1).rev() {
            if let Some(s) = slots.iter_mut().find(|s| &s.manifest.id == dep_id) {
                if let Some(scope) = s.scope.take() {
                    scope.dispose();
                }
                s.state = PluginState::Pending;
            }
        }
        let slot = slots.remove(pos);
        if let Some(scope) = slot.scope {
            scope.dispose();
        }
        tracing::info!("插件 {id} 已卸载");
        Ok(())
    }
}
