use std::sync::Arc;

use async_trait::async_trait;
use leo_core::ast::{ScriptDocument, SegmentKind};
use leo_core::context::Context;
use leo_core::error::{CoreError, CoreResult};
use leo_core::plugin::{Exporter, Plugin, PluginCategory, PluginManifest};

pub struct CsvExporterPlugin;

impl Default for CsvExporterPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl CsvExporterPlugin {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Plugin for CsvExporterPlugin {
    fn manifest(&self) -> PluginManifest {
        let mut manifest = PluginManifest::new(
            "leo-plugin-exporter-csv",
            "CSV 剧本台账导出插件",
            "0.1.0",
            PluginCategory::Exporter,
        );
        manifest.description = "将剧本中各章节的对白与旁白片段导出为配音录制台账 CSV。".into();
        manifest.capabilities = vec!["exporter:csv".into()];
        manifest
    }

    async fn apply(&self, ctx: &Context) -> CoreResult<()> {
        let exporter: Arc<dyn Exporter> = Arc::new(CsvExporter);
        ctx.provide(exporter).await;
        tracing::info!("已挂载 CsvExporterPlugin，提供 Exporter(csv) 服务");
        Ok(())
    }
}

pub struct CsvExporter;

#[async_trait]
impl Exporter for CsvExporter {
    fn format_name(&self) -> &str {
        "csv"
    }

    fn mime_type(&self) -> &str {
        "text/csv; charset=utf-8"
    }

    async fn export(&self, script: &ScriptDocument) -> CoreResult<Vec<u8>> {
        let mut buffer = Vec::new();
        // 写入 UTF-8 BOM 方便 Excel 打开不乱码
        buffer.extend_from_slice(b"\xEF\xBB\xBF");

        let mut writer = csv::WriterBuilder::new().from_writer(buffer);

        writer
            .write_record(&[
                "章节序号",
                "章节标题",
                "句子序号",
                "片段序号",
                "片段类型",
                "角色说话人",
                "台词/旁白内容",
                "音频切片ID",
                "预估时长(毫秒)",
            ])
            .map_err(|e| CoreError::Internal(format!("写入 CSV 表头失败: {e}")))?;

        for ch in &script.chapters {
            for p in &ch.paragraphs {
                for s in &p.sentences {
                    for seg in &s.segments {
                        let kind_label = match seg.kind {
                            SegmentKind::Narration => "旁白",
                            SegmentKind::Dialogue => "台词",
                            SegmentKind::Sfx => "音效",
                        };

                        let speaker = seg.speaker_id.as_deref().unwrap_or("未绑定");
                        let clip_id = seg.audio.as_ref().map(|a| a.clip_id.as_str()).unwrap_or("");
                        let duration = seg
                            .audio
                            .as_ref()
                            .map(|a| a.duration_ms.to_string())
                            .unwrap_or_default();

                        writer
                            .write_record(&[
                                &ch.index.to_string(),
                                &ch.title,
                                &s.index.to_string(),
                                &seg.index.to_string(),
                                kind_label,
                                speaker,
                                &seg.text,
                                clip_id,
                                &duration,
                            ])
                            .map_err(|e| CoreError::Internal(format!("写入 CSV 记录失败: {e}")))?;
                    }
                }
            }
        }

        writer
            .into_inner()
            .map_err(|e| CoreError::Internal(format!("刷新 CSV 缓冲区失败: {e}")))
    }
}
