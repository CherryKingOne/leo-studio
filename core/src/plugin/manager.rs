use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::context::Context;
use crate::error::{CoreError, CoreResult};
use crate::plugin::manifest::PluginManifest;
use crate::plugin::traits::Plugin;

/// 插件装配失败时的处理策略。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FailurePolicy {
    /// 任一插件缺少依赖或挂载失败即中止，并回滚本次已挂载的插件（默认）
    #[default]
    FailFast,
    /// 跳过失败插件及依赖它的插件，其余插件照常挂载；失败记录可通过 `failures()` 查询
    SkipFailed,
}

impl FailurePolicy {
    /// 解析配置值：`skip` / `skip_failed` 为跳过，其余均为快速失败
    pub fn parse(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "skip" | "skip_failed" | "skip-failed" => Self::SkipFailed,
            _ => Self::FailFast,
        }
    }
}

/// 单个插件的装配失败记录
#[derive(Debug, Clone)]
pub struct PluginFailure {
    pub plugin_id: String,
    pub error: CoreError,
}

#[derive(Default)]
struct ManagerState {
    plugins: Vec<Arc<dyn Plugin>>,
    loaded: Vec<String>,
    failures: Vec<PluginFailure>,
}

/// 插件管理器：负责插件的注册、按依赖拓扑序挂载、卸载与查询，严格保持注册顺序。
#[derive(Default, Clone)]
pub struct PluginManager {
    state: Arc<RwLock<ManagerState>>,
    policy: FailurePolicy,
}

impl PluginManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// 以指定失败策略创建插件管理器
    pub fn with_failure_policy(policy: FailurePolicy) -> Self {
        Self {
            state: Arc::default(),
            policy,
        }
    }

    pub fn failure_policy(&self) -> FailurePolicy {
        self.policy
    }

    /// 注册一个插件；插件 ID 重复时返回冲突错误，不会覆盖已注册插件
    pub async fn register(&self, plugin: Arc<dyn Plugin>) -> CoreResult<()> {
        let id = plugin.manifest().id;
        let mut state = self.state.write().await;
        if state.plugins.iter().any(|p| p.manifest().id == id) {
            return Err(CoreError::PluginConflict(id));
        }
        state.plugins.push(plugin);
        Ok(())
    }

    /// 按注册顺序获取所有已注册插件的清单列表
    pub async fn list_manifests(&self) -> Vec<PluginManifest> {
        let state = self.state.read().await;
        state.plugins.iter().map(|p| p.manifest()).collect()
    }

    /// 按挂载顺序返回当前已挂载的插件 ID
    pub async fn loaded_ids(&self) -> Vec<String> {
        self.state.read().await.loaded.clone()
    }

    /// 最近一次 `apply_all` 中被跳过的插件（仅 `SkipFailed` 策略下会有记录）
    pub async fn failures(&self) -> Vec<PluginFailure> {
        self.state.read().await.failures.clone()
    }

    /// 按依赖拓扑序（同层保持注册顺序）挂载所有尚未挂载的插件
    pub async fn apply_all(&self, ctx: &Context) -> CoreResult<()> {
        let (plugins, already_loaded) = {
            let state = self.state.read().await;
            (state.plugins.clone(), state.loaded.clone())
        };

        let mut failures = Vec::new();
        let mut failed: HashSet<String> = HashSet::new();
        let order = match self.resolve_order(&plugins, &mut failures, &mut failed) {
            Ok(order) => order,
            Err(err) => {
                tracing::error!("插件依赖解析失败: {err}");
                return Err(err);
            }
        };

        let mut loaded: HashSet<String> = already_loaded.iter().cloned().collect();
        let mut applied_now: Vec<Arc<dyn Plugin>> = Vec::new();

        for plugin in order {
            let manifest = plugin.manifest();
            if loaded.contains(&manifest.id) || failed.contains(&manifest.id) {
                continue;
            }

            let result = match manifest
                .dependencies
                .iter()
                .find(|dep| !loaded.contains(*dep))
            {
                Some(dep) => Err(CoreError::DependencyFailed {
                    plugin_id: manifest.id.clone(),
                    dependency: dep.clone(),
                }),
                None => {
                    tracing::info!("正在加载插件: {} ({})", manifest.name, manifest.id);
                    plugin.apply(&ctx.for_plugin(&manifest.id)).await
                }
            };

            match result {
                Ok(()) => {
                    loaded.insert(manifest.id.clone());
                    self.state.write().await.loaded.push(manifest.id.clone());
                    applied_now.push(plugin);
                }
                Err(err) => {
                    ctx.services.write().await.remove_owned_by(&manifest.id);
                    let err = CoreError::Plugin {
                        plugin_id: manifest.id.clone(),
                        message: format!("挂载失败：{err}"),
                    };
                    match self.policy {
                        FailurePolicy::FailFast => {
                            tracing::error!("{err}，正在回滚本次已挂载的插件");
                            for applied in applied_now.iter().rev() {
                                let id = applied.manifest().id;
                                if let Err(rollback_err) = self.dispose_plugin(applied, ctx).await {
                                    tracing::error!("回滚插件 {id} 失败: {rollback_err}");
                                }
                            }
                            return Err(err);
                        }
                        FailurePolicy::SkipFailed => {
                            tracing::error!("{err}，按 SkipFailed 策略跳过");
                            failed.insert(manifest.id.clone());
                            failures.push(PluginFailure {
                                plugin_id: manifest.id.clone(),
                                error: err,
                            });
                        }
                    }
                }
            }
        }

        self.state.write().await.failures = failures;
        Ok(())
    }

    /// 卸载一个已挂载的插件：调用其 `dispose` 并撤销它注册的全部服务。
    /// 仍被其他已挂载插件依赖时拒绝卸载。
    pub async fn unload(&self, plugin_id: &str, ctx: &Context) -> CoreResult<()> {
        let plugin = {
            let state = self.state.read().await;
            if !state.loaded.iter().any(|id| id == plugin_id) {
                return Err(CoreError::PluginNotFound(plugin_id.to_string()));
            }
            if let Some(dependent) = state.plugins.iter().map(|p| p.manifest()).find(|m| {
                state.loaded.contains(&m.id) && m.dependencies.iter().any(|d| d == plugin_id)
            }) {
                return Err(CoreError::Plugin {
                    plugin_id: plugin_id.to_string(),
                    message: format!("仍被已挂载插件 {} 依赖，无法卸载", dependent.id),
                });
            }
            state
                .plugins
                .iter()
                .find(|p| p.manifest().id == plugin_id)
                .cloned()
                .ok_or_else(|| CoreError::PluginNotFound(plugin_id.to_string()))?
        };
        self.dispose_plugin(&plugin, ctx).await
    }

    /// 按挂载的逆序卸载全部插件；单个插件卸载失败不会中断其余插件，返回第一个错误
    pub async fn dispose_all(&self, ctx: &Context) -> CoreResult<()> {
        let plugins: Vec<Arc<dyn Plugin>> = {
            let state = self.state.read().await;
            state
                .loaded
                .iter()
                .rev()
                .filter_map(|id| state.plugins.iter().find(|p| &p.manifest().id == id).cloned())
                .collect()
        };
        let mut first_err = None;
        for plugin in plugins {
            if let Err(err) = self.dispose_plugin(&plugin, ctx).await {
                tracing::error!("卸载插件 {} 失败: {err}", plugin.manifest().id);
                first_err.get_or_insert(err);
            }
        }
        first_err.map_or(Ok(()), Err)
    }

    async fn dispose_plugin(&self, plugin: &Arc<dyn Plugin>, ctx: &Context) -> CoreResult<()> {
        let id = plugin.manifest().id;
        let result = plugin.dispose(&ctx.for_plugin(&id)).await;
        ctx.services.write().await.remove_owned_by(&id);
        self.state.write().await.loaded.retain(|loaded| loaded != &id);
        tracing::info!("已卸载插件: {id}");
        result
    }

    /// 稳定拓扑排序：每轮按注册顺序挑出依赖已全部就绪的插件
    fn resolve_order(
        &self,
        plugins: &[Arc<dyn Plugin>],
        failures: &mut Vec<PluginFailure>,
        failed: &mut HashSet<String>,
    ) -> CoreResult<Vec<Arc<dyn Plugin>>> {
        let ids: HashSet<String> = plugins.iter().map(|p| p.manifest().id).collect();

        for plugin in plugins {
            let manifest = plugin.manifest();
            if let Some(dep) = manifest.dependencies.iter().find(|dep| !ids.contains(*dep)) {
                let err = CoreError::DependencyMissing {
                    plugin_id: manifest.id.clone(),
                    dependency: dep.clone(),
                };
                if self.policy == FailurePolicy::FailFast {
                    return Err(err);
                }
                tracing::error!("{err}，按 SkipFailed 策略跳过");
                failed.insert(manifest.id.clone());
                failures.push(PluginFailure {
                    plugin_id: manifest.id,
                    error: err,
                });
            }
        }

        let mut placed: HashSet<String> = HashSet::new();
        let mut order = Vec::with_capacity(plugins.len());
        let mut pending: Vec<Arc<dyn Plugin>> = plugins.to_vec();
        loop {
            let ready = pending.iter().position(|p| {
                p.manifest()
                    .dependencies
                    .iter()
                    .all(|dep| placed.contains(dep) || !ids.contains(dep))
            });
            let Some(index) = ready else { break };
            let plugin = pending.remove(index);
            placed.insert(plugin.manifest().id);
            order.push(plugin);
        }

        if !pending.is_empty() {
            let cycle: Vec<String> = pending.iter().map(|p| p.manifest().id).collect();
            let err = CoreError::DependencyCycle(cycle.join(" -> "));
            if self.policy == FailurePolicy::FailFast {
                return Err(err);
            }
            tracing::error!("{err}，按 SkipFailed 策略跳过");
            for id in cycle {
                failed.insert(id.clone());
                failures.push(PluginFailure {
                    plugin_id: id,
                    error: err.clone(),
                });
            }
        }

        Ok(order)
    }
}
