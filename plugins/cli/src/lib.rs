pub mod cli;

use std::sync::Arc;

use async_trait::async_trait;
use leo_core::context::Context;
use leo_core::error::CoreResult;
use leo_core::plugin::{Plugin, PluginCategory, PluginManifest, PluginManager};
use leo_plugin_cleaner_kaomoji::KaomojiCleanerPlugin;
use leo_plugin_dialogue_quotes::DialogueQuotesPlugin;
use leo_plugin_exporter_csv::CsvExporterPlugin;
use leo_plugin_exporter_json::JsonExporterPlugin;
use leo_plugin_importer_txt::TxtImporterPlugin;
use leo_plugin_parser_novel_regex::NovelRegexParserPlugin;
use leo_plugin_storage_csv::CsvStoragePlugin;
use leo_plugin_tts_edge::EdgeTtsPlugin;

pub use cli::{
    Cli, Commands, SynthesisOptions, SynthesisReport, execute_cli, execute_cli_with, list_projects,
    synthesize_chapter,
};

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

/// 装配并初始化 CLI 执行环境下的上下文与全量插件
pub async fn setup_cli_runtime(data_dir: &str) -> CoreResult<(Context, Arc<PluginManager>)> {
    let ctx = Context::new("leo_cli");
    let manager = Arc::new(PluginManager::new());

    manager.register(Arc::new(KaomojiCleanerPlugin::new())).await?;
    manager.register(Arc::new(NovelRegexParserPlugin::new())).await?;
    manager.register(Arc::new(DialogueQuotesPlugin::new())).await?;
    manager.register(Arc::new(CsvStoragePlugin::new(data_dir))).await?;
    manager.register(Arc::new(EdgeTtsPlugin::new())).await?;
    manager.register(Arc::new(TxtImporterPlugin::new())).await?;
    manager.register(Arc::new(CsvExporterPlugin::new())).await?;
    manager.register(Arc::new(JsonExporterPlugin::new())).await?;
    manager.register(Arc::new(CliPlugin::new())).await?;

    manager.apply_all(&ctx).await?;
    Ok((ctx, manager))
}
