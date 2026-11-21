//! leo-plugin-exchange：导入 / 导出**注册层**插件。
//!
//! - 导入器子插件：`ctx.register::<dyn Importer>("txt", ..)`；本插件按文件扩展名挑选导入器。
//! - 导出器子插件：`ctx.register::<dyn Exporter>("json", ..)`；本插件按格式名挑选导出器。
//!
//! 以后加 epub / docx 导入、srt / 剪映工程导出，只要新增子插件并在 catalog 里登记，接口层自动可用。
//!
//! 配置：`{ "default_exporter": "json" }`

use std::sync::Arc;

use async_trait::async_trait;
use leo_core::context::Context;
use leo_core::error::{CoreError, CoreResult};
use leo_core::plugin::{Plugin, PluginCategory, PluginManifest};
use leo_core::registry::{DriverInfo, Registry};
use leo_core::spi::{Exporter, Importer};
use serde::{Deserialize, Serialize};

pub const PLUGIN_ID: &str = "leo-plugin-exchange";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ExchangeRegistryConfig {
    /// 不指定格式时用的导出器
    pub default_exporter: Option<String>,
    /// 多个导入器支持同一扩展名时优先用的导入器
    pub default_importer: Option<String>,
}

#[derive(Default)]
pub struct ExchangeRegistryPlugin {
    config: ExchangeRegistryConfig,
}

impl ExchangeRegistryPlugin {
    pub fn new(config: ExchangeRegistryConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl Plugin for ExchangeRegistryPlugin {
    fn manifest(&self) -> PluginManifest {
        PluginManifest::new(PLUGIN_ID, "导入导出注册层", env!("CARGO_PKG_VERSION"), PluginCategory::Registry)
            .with_description("管理全部导入器 / 导出器子插件，按扩展名挑选导入器、按格式挑选导出器。")
            .with_capabilities(["registry:importer", "registry:exporter"])
    }

    async fn apply(&self, ctx: &Context) -> CoreResult<()> {
        let importers = ctx.registry::<dyn Importer>();
        let exporters = ctx.registry::<dyn Exporter>();
        importers.set_default(self.config.default_importer.as_deref());
        exporters.set_default(self.config.default_exporter.as_deref());
        let (i, e) = (importers.clone(), exporters.clone());
        ctx.on_dispose(move || {
            i.set_default(None);
            e.set_default(None);
        });
        ctx.provide(ExchangeHub { importers, exporters }).await;
        Ok(())
    }
}

/// 导出格式描述
#[derive(Debug, Clone, Serialize)]
pub struct ExportFormat {
    pub id: String,
    pub format: String,
    pub mime_type: String,
    pub extension: String,
}

#[derive(Clone)]
pub struct ExchangeHub {
    importers: Registry<dyn Importer>,
    exporters: Registry<dyn Exporter>,
}

fn extension_of(filename: &str) -> String {
    filename.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).unwrap_or_default()
}

impl ExchangeHub {
    pub async fn of(ctx: &Context) -> Self {
        match ctx.inject::<ExchangeHub>().await {
            Ok(hub) => (*hub).clone(),
            Err(_) => ExchangeHub { importers: ctx.registry::<dyn Importer>(), exporters: ctx.registry::<dyn Exporter>() },
        }
    }

    pub fn importer_drivers(&self) -> Vec<DriverInfo> {
        self.importers.describe()
    }

    pub fn exporter_drivers(&self) -> Vec<DriverInfo> {
        self.exporters.describe()
    }

    /// 所有导入器支持的扩展名（小写、不带点、去重）
    pub fn import_extensions(&self) -> Vec<String> {
        let mut exts: Vec<String> = self
            .importers
            .entries()
            .iter()
            .flat_map(|(_, i)| i.supported_extensions().iter().map(|e| e.trim_start_matches('.').to_ascii_lowercase()))
            .collect();
        exts.dedup();
        exts.sort();
        exts.dedup();
        exts
    }

    /// 按文件名挑选导入器：默认导入器支持该扩展名就用它，否则用最早注册的支持者
    pub fn importer_for(&self, filename: &str) -> CoreResult<(String, Arc<dyn Importer>)> {
        let ext = extension_of(filename);
        let supports = |i: &Arc<dyn Importer>| i.supported_extensions().iter().any(|e| e.trim_start_matches('.').eq_ignore_ascii_case(&ext));
        if let Some((id, imp)) = self.importers.default_entry() {
            if supports(&imp) {
                return Ok((id, imp));
            }
        }
        self.importers.entries().into_iter().find(|(_, i)| supports(i)).ok_or_else(|| {
            CoreError::InvalidParameter(format!(
                "没有能导入 .{ext} 文件的插件（支持：{}）",
                self.import_extensions().iter().map(|e| format!(".{e}")).collect::<Vec<_>>().join(", ")
            ))
        })
    }

    /// 按格式挑选导出器：先按驱动 id，再按导出器自报的格式名；`None` 用默认导出器
    pub fn exporter(&self, format: Option<&str>) -> CoreResult<(String, Arc<dyn Exporter>)> {
        let Some(format) = format else { return self.exporters.resolve(None) };
        let format = format.to_ascii_lowercase();
        if let Some(e) = self.exporters.get(&format) {
            return Ok((format, e));
        }
        self.exporters
            .entries()
            .into_iter()
            .find(|(_, e)| e.format_name().eq_ignore_ascii_case(&format))
            .ok_or_else(|| {
                CoreError::InvalidParameter(format!(
                    "不支持的导出格式 {format}（支持：{}）",
                    self.export_formats().iter().map(|f| f.id.clone()).collect::<Vec<_>>().join(", ")
                ))
            })
    }

    pub fn export_formats(&self) -> Vec<ExportFormat> {
        self.exporters
            .entries()
            .into_iter()
            .map(|(id, e)| ExportFormat {
                id,
                format: e.format_name().to_string(),
                mime_type: e.mime_type().to_string(),
                extension: e.file_extension().to_string(),
            })
            .collect()
    }
}
