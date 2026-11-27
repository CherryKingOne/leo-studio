use std::fs;
use std::path::Path;

use leo_core::context::Context;
use leo_core::plugin::PluginManager;
use leo_plugin_exchange::ExchangeHub;
use leo_plugin_storage::StorageHub;

use super::CliResult;

pub async fn run(
    ctx: &Context,
    _manager: &PluginManager,
    file: &Path,
    title: Option<&str>,
) -> CliResult<()> {
    let bytes = fs::read(file)?;
    let filename = file
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("novel.txt");
    let (_, importer) = ExchangeHub::of(ctx).await.importer_for(filename)?;
    let mut doc = importer.import(filename, &bytes).await?;
    if let Some(title) = title {
        doc.title = title.to_string();
    }
    if let Ok(storage) = StorageHub::of(ctx).await.default_storage() {
        storage.save_script(&doc).await?;
    }
    println!("项目 ID: {}", doc.id);
    println!("标题: {}", doc.title);
    println!("章节数: {}", doc.chapters.len());
    println!("总字数: {}", doc.total_words);
    Ok(())
}
