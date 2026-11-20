use std::path::Path;
use std::sync::Arc;

use leo_core::context::Context;
use leo_core::error::CoreResult;
use leo_core::plugin::PluginManager;
use leo_plugin_cleaner_kaomoji::KaomojiCleanerPlugin;
use leo_plugin_dialogue_quotes::DialogueQuotesPlugin;
use leo_plugin_exporter_csv::CsvExporterPlugin;
use leo_plugin_exporter_json::JsonExporterPlugin;
use leo_plugin_importer_txt::TxtImporterPlugin;
use leo_plugin_parser_novel_regex::NovelRegexParserPlugin;
use leo_plugin_storage_csv::CsvStoragePlugin;
use leo_plugin_tts_edge::EdgeTtsPlugin;

/// 初始化并挂载默认官方内置插件
pub async fn setup_default_plugins(
    ctx: &Context,
    manager: &PluginManager,
    csv_dir: &Path,
) -> CoreResult<()> {
    manager.register(Arc::new(KaomojiCleanerPlugin::new())).await;
    manager.register(Arc::new(NovelRegexParserPlugin::new())).await;
    manager.register(Arc::new(DialogueQuotesPlugin::new())).await;
    manager.register(Arc::new(CsvStoragePlugin::new(csv_dir))).await;
    manager.register(Arc::new(EdgeTtsPlugin::new())).await;
    manager.register(Arc::new(TxtImporterPlugin::new())).await;
    manager.register(Arc::new(JsonExporterPlugin::new())).await;
    manager.register(Arc::new(CsvExporterPlugin::new())).await;

    manager.apply_all(ctx).await?;
    tracing::info!("官方内置插件已全部装配完毕");
    Ok(())
}
