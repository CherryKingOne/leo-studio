pub mod cli;
pub mod commands;
pub mod output;
pub mod runtime;

use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
pub use cli::{
    ChapterCommands, CharacterCommands, Cli, Commands, PolyphoneCommands, ProjectCommands,
    RecycleBinCommands, execute_cli,
};
use leo_core::context::Context;
use leo_core::error::CoreResult;
use leo_core::plugin::{Plugin, PluginCategory, PluginManager, PluginManifest};
use leo_plugin_catalog::{HostEnv, PluginsConfig};

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
            "提供 v3 项目、章节、角色、多音字、回收站管理命令，并保留 AST 导入导出与语音生产命令。"
                .into();
        manifest.capabilities = vec![
            "cli:projects".into(),
            "cli:chapters".into(),
            "cli:characters".into(),
            "cli:polyphones".into(),
            "cli:recycle-bin".into(),
            "cli:import".into(),
            "cli:export".into(),
            "cli:synthesize".into(),
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
    let env = HostEnv {
        csv_dir: Path::new(data_dir).join(".leo-studio"),
    };
    let ctx = Context::new("leo_cli");
    let manager = Arc::new(PluginManager::new());
    manager.register(Arc::new(CliPlugin::new())).await?;
    leo_plugin_catalog::install(&ctx, &manager, &env, &config).await?;
    Ok((ctx, manager))
}
