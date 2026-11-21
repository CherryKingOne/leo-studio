//! leo-plugin-storage：存储**注册层**插件。
//!
//! 子插件（csv，以后的 sqlite / postgres / s3……）用 `ctx.register::<dyn StorageProvider>("csv", ..)` 注册，
//! 本插件按配置选出默认存储，对外提供 [`StorageHub`]。业务代码只问 `StorageHub` 要存储，不关心是哪种。
//!
//! 配置：`{ "default": "csv" }`

use std::sync::Arc;

use async_trait::async_trait;
use leo_core::context::Context;
use leo_core::error::CoreResult;
use leo_core::plugin::{Plugin, PluginCategory, PluginManifest};
use leo_core::registry::{DriverInfo, Registry};
use leo_core::spi::StorageProvider;
use serde::{Deserialize, Serialize};

pub const PLUGIN_ID: &str = "leo-plugin-storage";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct StorageRegistryConfig {
    /// 默认存储驱动 id
    pub default: Option<String>,
}

#[derive(Default)]
pub struct StorageRegistryPlugin {
    config: StorageRegistryConfig,
}

impl StorageRegistryPlugin {
    pub fn new(config: StorageRegistryConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl Plugin for StorageRegistryPlugin {
    fn manifest(&self) -> PluginManifest {
        PluginManifest::new(PLUGIN_ID, "存储注册层", env!("CARGO_PKG_VERSION"), PluginCategory::Registry)
            .with_description("管理全部存储驱动子插件，按配置挑选默认存储。")
            .with_capabilities(["registry:storage"])
    }

    async fn apply(&self, ctx: &Context) -> CoreResult<()> {
        let registry = ctx.registry::<dyn StorageProvider>();
        registry.set_default(self.config.default.as_deref());
        let r = registry.clone();
        ctx.on_dispose(move || r.set_default(None));
        ctx.provide(StorageHub { registry }).await;
        Ok(())
    }
}

#[derive(Clone)]
pub struct StorageHub {
    registry: Registry<dyn StorageProvider>,
}

impl StorageHub {
    pub async fn of(ctx: &Context) -> Self {
        match ctx.inject::<StorageHub>().await {
            Ok(hub) => (*hub).clone(),
            Err(_) => StorageHub { registry: ctx.registry::<dyn StorageProvider>() },
        }
    }

    pub fn drivers(&self) -> Vec<DriverInfo> {
        self.registry.describe()
    }

    /// 默认存储
    pub fn default_storage(&self) -> CoreResult<Arc<dyn StorageProvider>> {
        self.registry.resolve(None).map(|(_, s)| s)
    }

    /// 指定存储；`None` 取默认
    pub fn storage(&self, id: Option<&str>) -> CoreResult<Arc<dyn StorageProvider>> {
        self.registry.resolve(id).map(|(_, s)| s)
    }
}
