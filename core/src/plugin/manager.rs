use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::context::Context;
use crate::error::CoreResult;
use crate::plugin::manifest::PluginManifest;
use crate::plugin::traits::Plugin;

/// 插件管理器：负责插件生命周期的注册、挂载与查询。
#[derive(Default, Clone)]
pub struct PluginManager {
    plugins: Arc<RwLock<HashMap<String, Arc<dyn Plugin>>>>,
}

impl PluginManager {
    pub fn new() -> Self {
        Self {
            plugins: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 注册一个插件
    pub async fn register(&self, plugin: Arc<dyn Plugin>) {
        let manifest = plugin.manifest();
        let mut map = self.plugins.write().await;
        map.insert(manifest.id, plugin);
    }

    /// 获取所有已注册插件的清单列表
    pub async fn list_manifests(&self) -> Vec<PluginManifest> {
        let map = self.plugins.read().await;
        map.values().map(|p| p.manifest()).collect()
    }

    /// 按配置将所有插件激活挂载到 Context 上
    pub async fn apply_all(&self, ctx: &Context) -> CoreResult<()> {
        let plugins: Vec<Arc<dyn Plugin>> = {
            let map = self.plugins.read().await;
            map.values().cloned().collect()
        };

        for plugin in plugins {
            let manifest = plugin.manifest();
            tracing::info!("正在加载插件: {} ({})", manifest.name, manifest.id);
            plugin.apply(ctx).await?;
        }
        Ok(())
    }
}
