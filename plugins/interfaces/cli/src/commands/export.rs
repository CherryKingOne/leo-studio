use std::fs;
use std::path::Path;

use leo_core::context::Context;
use leo_plugin_exchange::ExchangeHub;
use leo_plugin_storage::StorageHub;

use super::CliResult;

pub async fn run(
    ctx: &Context,
    project_id: &str,
    format: &str,
    output: Option<&Path>,
) -> CliResult<()> {
    let storage = StorageHub::of(ctx)
        .await
        .default_storage()
        .map_err(|err| format!("未挂载存储插件: {err}"))?;
    let doc = storage
        .load_script(project_id)
        .await?
        .ok_or_else(|| format!("未找到项目 ID: {project_id}"))?;
    let (_, exporter) = ExchangeHub::of(ctx).await.exporter(Some(format))?;
    let bytes = exporter.export(&doc).await?;
    let ext = exporter.file_extension().to_string();
    let target = output
        .map(Path::to_path_buf)
        .unwrap_or_else(|| format!("{}_{}.{}", doc.title, doc.id, ext).into());
    fs::write(&target, &bytes)?;
    println!("导出文件: {}", target.display());
    println!("格式: {ext}");
    println!("大小: {} 字节", bytes.len());
    Ok(())
}
