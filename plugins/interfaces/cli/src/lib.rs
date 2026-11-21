pub mod cli;

use std::sync::Arc;

use async_trait::async_trait;
use leo_core::context::Context;
use leo_core::error::CoreResult;
use leo_core::plugin::{Plugin, PluginCategory, PluginManifest, PluginManager};
use leo_plugin_catalog::{HostEnv, PluginsConfig};
pub use cli::{Cli, Commands, execute_cli};

/// Leo Studio 命令行交互插件
pub struct CliPlugin;

impl Default for CliPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl CliPlugin {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Plugin for CliPlugin {
    fn manifest(&self) -> PluginManifest {
        let mut manifest = PluginManifest::new(
            "leo-plugin-cli",
            "Leo Studio 命令行 CLI 交互插件",
            "0.2.0",
            PluginCategory::Cli,
        );
        manifest.description =
            "提供全功能命令行操作支持，包括小说导入解析、剧本导出、声线查询、章节语音合成与工程管理。".into();
        manifest.capabilities = vec![
            "cli:import".into(),
            "cli:export".into(),
            "cli:synthesize".into(),
            "cli:management".into(),
        ];
        manifest
    }

    async fn apply(&self, _ctx: &Context) -> CoreResult<()> {
        tracing::info!("已挂载 CliPlugin");
        Ok(())
    }
}

/// 装配并初始化 CLI 执行环境下的上下文与全量插件。
///
/// 插件列表来自 leo-plugin-catalog（启用 / 停用、默认驱动见 `leo-plugins.json`），CLI 本身不认识具体插件。
pub async fn setup_cli_runtime(data_dir: &str) -> CoreResult<(Context, Arc<PluginManager>)> {
    let config = PluginsConfig::load(None)?;
    let env = HostEnv { csv_dir: data_dir.into() };
    let ctx = Context::new("leo_cli");
    let manager = Arc::new(PluginManager::new());
    manager.register(Arc::new(CliPlugin::new())).await?;
    leo_plugin_catalog::install(&ctx, &manager, &env, &config).await?;
    Ok((ctx, manager))
}
