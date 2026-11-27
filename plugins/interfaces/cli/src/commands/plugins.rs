use leo_core::plugin::PluginManager;

use super::CliResult;

pub async fn run(manager: &PluginManager) -> CliResult<()> {
    let plugins = manager.statuses().await;
    println!("插件总数: {}", plugins.len());
    for plugin in plugins {
        println!(
            "{} | {} | {} | {} | {}",
            plugin.manifest.id,
            plugin.manifest.version,
            format!("{:?}", plugin.manifest.category).to_lowercase(),
            format!("{:?}", plugin.state).to_lowercase(),
            plugin.manifest.capabilities.join(",")
        );
    }
    Ok(())
}
