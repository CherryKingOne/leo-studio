//! leo-plugin-catalog：插件目录。
//!
//! Rust 没有运行时动态加载（不打算引入 dylib / wasm 的复杂度），所以“有哪些插件”需要在编译期列出来。
//! 这个 crate 就是**唯一**列插件的地方：
//!
//! - 新增一个子插件：在 `Cargo.toml` 加一行依赖，在下面的 [`CATALOG`] 加一项。内核、注册层、接口层都不用改。
//! - 启用 / 停用插件、选默认驱动：改 `leo-plugins.json`（不用重新编译）。
//!
//! ```json
//! {
//!   "disabled": ["leo-plugin-exporter-csv"],
//!   "text":     { "cleaners": ["kaomoji"], "chapter_splitter": "novel-regex", "dialogue_splitter": "quotes" },
//!   "exchange": { "default_exporter": "json" },
//!   "storage":  { "default": "csv" },
//!   "tts":      { "default": "edge" }
//! }
//! ```

use std::path::{Path, PathBuf};
use std::sync::Arc;

use leo_core::context::Context;
use leo_core::error::{CoreError, CoreResult};
use leo_core::plugin::{Plugin, PluginManager};
use serde::{Deserialize, Serialize};

pub use leo_plugin_exchange::{ExchangeHub, ExchangeRegistryConfig};
pub use leo_plugin_storage::{StorageHub, StorageRegistryConfig};
pub use leo_plugin_text::{TextHub, TextRegistryConfig};
pub use leo_plugin_tts::{TtsHub, TtsRegistryConfig};

/// 默认配置文件名（放在工作目录，或用环境变量 `LEO_PLUGINS_CONFIG` 指定路径）
pub const CONFIG_FILE: &str = "leo-plugins.json";

/// `leo-plugins.json`
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PluginsConfig {
    /// 停用的插件 id
    pub disabled: Vec<String>,
    pub text: TextRegistryConfig,
    pub exchange: ExchangeRegistryConfig,
    pub storage: StorageRegistryConfig,
    pub tts: TtsRegistryConfig,
}

impl PluginsConfig {
    /// 读取配置：`path` > 环境变量 `LEO_PLUGINS_CONFIG` > 当前目录的 `leo-plugins.json`；文件不存在时用默认配置
    pub fn load(path: Option<&Path>) -> CoreResult<Self> {
        let path: Option<PathBuf> = path
            .map(Path::to_path_buf)
            .or_else(|| std::env::var_os("LEO_PLUGINS_CONFIG").map(PathBuf::from))
            .or_else(|| Some(PathBuf::from(CONFIG_FILE)).filter(|p| p.exists()));
        let Some(path) = path else { return Ok(Self::default()) };
        let raw = std::fs::read_to_string(&path)
            .map_err(|e| CoreError::InvalidParameter(format!("读取插件配置 {} 失败：{e}", path.display())))?;
        let config: Self = serde_json::from_str(&raw)
            .map_err(|e| CoreError::InvalidParameter(format!("插件配置 {} 格式错误：{e}", path.display())))?;
        tracing::info!("已读取插件配置 {}", path.display());
        Ok(config)
    }

    fn is_disabled(&self, id: &str) -> bool {
        self.disabled.iter().any(|d| d == id)
    }
}

/// 创建插件时需要的宿主环境
#[derive(Debug, Clone)]
pub struct HostEnv {
    /// CSV 存储目录
    pub csv_dir: PathBuf,
}

/// 目录中的一项：插件 id + 构造函数
pub struct CatalogEntry {
    pub id: &'static str,
    /// 注册层插件不能停用
    pub required: bool,
    pub build: fn(&HostEnv, &PluginsConfig) -> Arc<dyn Plugin>,
}

/// 编进程序的全部插件。顺序只影响“没有配置默认驱动时谁是默认”（最早注册的优先）。
pub const CATALOG: &[CatalogEntry] = &[
    // ---- 注册层（大箱子）----
    CatalogEntry { id: leo_plugin_text::PLUGIN_ID, required: true, build: |_, c| Arc::new(leo_plugin_text::TextRegistryPlugin::new(c.text.clone())) },
    CatalogEntry { id: leo_plugin_exchange::PLUGIN_ID, required: true, build: |_, c| Arc::new(leo_plugin_exchange::ExchangeRegistryPlugin::new(c.exchange.clone())) },
    CatalogEntry { id: leo_plugin_storage::PLUGIN_ID, required: true, build: |_, c| Arc::new(leo_plugin_storage::StorageRegistryPlugin::new(c.storage.clone())) },
    CatalogEntry { id: leo_plugin_tts::PLUGIN_ID, required: true, build: |_, c| Arc::new(leo_plugin_tts::TtsRegistryPlugin::new(c.tts.clone())) },
    // ---- 文本子插件 ----
    CatalogEntry { id: "leo-plugin-cleaner-kaomoji", required: false, build: |_, _| Arc::new(leo_plugin_cleaner_kaomoji::KaomojiCleanerPlugin::new()) },
    CatalogEntry { id: "leo-plugin-parser-novel-regex", required: false, build: |_, _| Arc::new(leo_plugin_parser_novel_regex::NovelRegexParserPlugin::new()) },
    CatalogEntry { id: "leo-plugin-dialogue-quotes", required: false, build: |_, _| Arc::new(leo_plugin_dialogue_quotes::DialogueQuotesPlugin::new()) },
    // ---- 导入导出子插件 ----
    CatalogEntry { id: "leo-plugin-importer-txt", required: false, build: |_, _| Arc::new(leo_plugin_importer_txt::TxtImporterPlugin::new()) },
    CatalogEntry { id: "leo-plugin-exporter-json", required: false, build: |_, _| Arc::new(leo_plugin_exporter_json::JsonExporterPlugin::new()) },
    CatalogEntry { id: "leo-plugin-exporter-csv", required: false, build: |_, _| Arc::new(leo_plugin_exporter_csv::CsvExporterPlugin::new()) },
    // ---- 存储子插件 ----
    CatalogEntry { id: "leo-plugin-storage-csv", required: false, build: |env, _| Arc::new(leo_plugin_storage_csv::CsvStoragePlugin::new(&env.csv_dir)) },
    // ---- 语音合成子插件 ----
    CatalogEntry { id: "leo-plugin-tts-edge", required: false, build: |_, _| Arc::new(leo_plugin_tts_edge::EdgeTtsPlugin::new()) },
];

/// 按配置挑出要启用的插件
pub fn enabled_plugins(env: &HostEnv, config: &PluginsConfig) -> CoreResult<Vec<Arc<dyn Plugin>>> {
    for id in &config.disabled {
        match CATALOG.iter().find(|e| e.id == id) {
            None => tracing::warn!("插件配置停用了不存在的插件 {id}，已忽略"),
            Some(e) if e.required => {
                return Err(CoreError::InvalidParameter(format!("{id} 是注册层插件，不能停用")));
            }
            Some(_) => {}
        }
    }
    let plugins = CATALOG
        .iter()
        .filter(|e| !config.is_disabled(e.id))
        .map(|e| {
            let p = (e.build)(env, config);
            debug_assert_eq!(p.manifest().id, e.id, "CATALOG 里的 id 必须与插件 manifest 一致");
            p
        })
        .collect();
    Ok(plugins)
}

/// 把启用的插件登记到管理器并装配
pub async fn install(ctx: &Context, manager: &PluginManager, env: &HostEnv, config: &PluginsConfig) -> CoreResult<()> {
    for plugin in enabled_plugins(env, config)? {
        manager.register(plugin).await?;
    }
    manager.apply_all(ctx).await?;
    tracing::info!("插件目录装配完毕");
    Ok(())
}

/// 便捷入口：新建上下文与管理器并装配全部启用插件
pub async fn bootstrap(scope: &str, env: &HostEnv, config: &PluginsConfig) -> CoreResult<(Context, Arc<PluginManager>)> {
    let ctx = Context::new(scope);
    let manager = Arc::new(PluginManager::new());
    install(&ctx, &manager, env, config).await?;
    Ok((ctx, manager))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_ids_match_manifests() {
        let env = HostEnv { csv_dir: std::env::temp_dir().join("leo-catalog-test") };
        let config = PluginsConfig::default();
        for e in CATALOG {
            assert_eq!((e.build)(&env, &config).manifest().id, e.id);
        }
    }

    #[test]
    fn registry_plugins_cannot_be_disabled() {
        let env = HostEnv { csv_dir: std::env::temp_dir() };
        let config = PluginsConfig { disabled: vec!["leo-plugin-tts".into()], ..Default::default() };
        assert!(enabled_plugins(&env, &config).is_err());
    }

    #[test]
    fn config_rejects_unknown_fields() {
        assert!(serde_json::from_str::<PluginsConfig>(r#"{"tts":{"defualt":"edge"}}"#).is_err());
        let c: PluginsConfig = serde_json::from_str(r#"{"tts":{"default":"edge"},"disabled":["leo-plugin-exporter-csv"]}"#).unwrap();
        assert_eq!(c.tts.default.as_deref(), Some("edge"));
    }
}
