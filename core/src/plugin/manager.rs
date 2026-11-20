use std::sync::Arc;
use tokio::sync::RwLock;

use crate::context::Context;
use crate::error::{CoreError, CoreResult};
use crate::plugin::manifest::PluginManifest;
use crate::plugin::traits::Plugin;

/// 插件管理器：负责插件生命周期的注册、挂载与查询，严格保持注册顺序。
#[derive(Default, Clone)]
pub struct PluginManager {
    plugins: Arc<RwLock<Vec<Arc<dyn Plugin>>>>,
}

impl PluginManager {
    pub fn new() -> Self {
        Self {
            plugins: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// 注册一个插件；插件 ID 重复时返回冲突错误，不会覆盖已注册插件
    pub async fn register(&self, plugin: Arc<dyn Plugin>) -> CoreResult<()> {
        let id = plugin.manifest().id;
        let mut plugins = self.plugins.write().await;
        if plugins.iter().any(|p| p.manifest().id == id) {
            return Err(CoreError::PluginConflict(id));
        }
        plugins.push(plugin);
        Ok(())
    }

    /// 按注册顺序获取所有已注册插件的清单列表
    pub async fn list_manifests(&self) -> Vec<PluginManifest> {
        let plugins = self.plugins.read().await;
        plugins.iter().map(|p| p.manifest()).collect()
    }

    /// 按注册顺序将所有插件激活挂载到 Context 上
    pub async fn apply_all(&self, ctx: &Context) -> CoreResult<()> {
        let plugins: Vec<Arc<dyn Plugin>> = self.plugins.read().await.clone();

        for plugin in plugins {
            let manifest = plugin.manifest();
            tracing::info!("正在加载插件: {} ({})", manifest.name, manifest.id);
            plugin.apply(ctx).await?;
        }
        Ok(())
    }
}
