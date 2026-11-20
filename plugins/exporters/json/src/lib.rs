use std::sync::Arc;

use async_trait::async_trait;
use leo_core::ast::ScriptDocument;
use leo_core::context::Context;
use leo_core::error::{CoreError, CoreResult};
use leo_core::plugin::{Exporter, Plugin, PluginCategory, PluginManifest};

pub struct JsonExporterPlugin;

impl Default for JsonExporterPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl JsonExporterPlugin {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Plugin for JsonExporterPlugin {
    fn manifest(&self) -> PluginManifest {
        let mut manifest = PluginManifest::new(
            "leo-plugin-exporter-json",
            "JSON 剧本导出插件",
            "0.1.0",
            PluginCategory::Exporter,
        );
        manifest.description = "将剧本 AST 结构、章节、分句、台词及音频切片引用导出为标准 JSON。".into();
        manifest.capabilities = vec!["exporter:json".into()];
        manifest
    }

    async fn apply(&self, ctx: &Context) -> CoreResult<()> {
        let exporter: Arc<dyn Exporter> = Arc::new(JsonExporter);
        ctx.provide_named("json", exporter).await?;
        tracing::info!("已挂载 JsonExporterPlugin，提供 Exporter(json) 服务");
        Ok(())
    }
}

pub struct JsonExporter;

#[async_trait]
impl Exporter for JsonExporter {
    fn format_name(&self) -> &str {
        "json"
    }

    fn mime_type(&self) -> &str {
        "application/json"
    }

    async fn export(&self, script: &ScriptDocument) -> CoreResult<Vec<u8>> {
        serde_json::to_vec_pretty(script)
            .map_err(|e| CoreError::Internal(format!("序列化 JSON 失败: {e}")))
    }
}
