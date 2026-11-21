use std::sync::Arc;

use async_trait::async_trait;
use leo_core::ast::{ChapterNode, ParagraphNode, ScriptDocument, SegmentNode, SentenceNode};
use leo_core::context::Context;
use leo_core::error::{CoreError, CoreResult};
use leo_core::plugin::{Plugin, PluginCategory, PluginManifest};
use leo_core::spi::Importer;
use leo_plugin_text::TextHub;

/// 本插件在注册表里的驱动 id
pub const DRIVER_ID: &str = "txt";

/// TXT 导入插件。
///
/// 只依赖文本注册层（`leo-plugin-text`）的 [`TextHub`]：用哪个清洗器、分章器、台词切分器由文本注册层的配置决定，
/// 本插件不再直接依赖任何具体的文本子插件。
pub struct TxtImporterPlugin;

impl Default for TxtImporterPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl TxtImporterPlugin {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Plugin for TxtImporterPlugin {
    fn manifest(&self) -> PluginManifest {
        PluginManifest::new("leo-plugin-importer-txt", "TXT 纯文本小说导入插件", "0.2.0", PluginCategory::Importer)
            .with_description("支持 .txt 格式纯文本小说解析，自动执行编码解码、标题识别、清洗分章并生成 Script AST。")
            .with_capabilities(["importer:txt"])
    }

    async fn apply(&self, ctx: &Context) -> CoreResult<()> {
        let importer: Arc<dyn Importer> = Arc::new(TxtImporter::new(ctx.clone()));
        ctx.register::<dyn Importer>(DRIVER_ID, importer)?;
        tracing::info!("已挂载 TxtImporterPlugin，已注册导入器 txt");
        Ok(())
    }
}

pub struct TxtImporter {
    ctx: Context,
}

impl TxtImporter {
    /// 每次导入时从文本注册层取清洗链、分章器、台词切分器（所以文本插件热替换后立即生效）
    pub fn new(ctx: Context) -> Self {
        Self { ctx }
    }

    /// 解码字节流（优先 UTF-8，失败尝试 GB18030/GBK）
    pub fn decode_bytes(bytes: &[u8]) -> String {
        if let Ok(text) = std::str::from_utf8(bytes) {
            return text.to_string();
        }
        let (cow, _, _) = encoding_rs::GB18030.decode(bytes);
        cow.to_string()
    }
}

#[async_trait]
impl Importer for TxtImporter {
    fn supported_extensions(&self) -> &[&str] {
        &["txt"]
    }

    async fn import(&self, filename: &str, content: &[u8]) -> CoreResult<ScriptDocument> {
        if content.is_empty() {
            return Err(CoreError::InvalidParameter("导入的 TXT 文件内容为空".into()));
        }

        let text = TextHub::of(&self.ctx).await;
        let chapter_splitter = text
            .chapter_splitter()
            .map_err(|e| CoreError::Pipeline(format!("TXT 导入需要分章插件：{e}")))?;
        let dialogue_splitter = text
            .dialogue_splitter()
            .map_err(|e| CoreError::Pipeline(format!("TXT 导入需要台词切分插件：{e}")))?;

        let raw_text = Self::decode_bytes(content);
        let (parsed_title, author, genre) = chapter_splitter.parse_filename(filename);

        let title = if parsed_title.is_empty() {
            filename.trim_end_matches(".txt").to_string()
        } else {
            parsed_title
        };

        let project_id = uuid::Uuid::new_v4().simple().to_string();
        let mut script = ScriptDocument::new(project_id, title, author, genre);

        // 1. 行级清洗
        let cleaned_lines: Vec<String> = raw_text
            .lines()
            .map(|l| text.clean_line(l))
            .filter(|l| !l.is_empty())
            .collect();
        let cleaned_text = cleaned_lines.join("\n");

        // 2. 章节切分
        let raw_chapters = chapter_splitter.split_chapters(&cleaned_text);
        let mut chapter_nodes = Vec::new();

        let mut segment_global_index = 0;
        for (c_idx, raw_ch) in raw_chapters.into_iter().enumerate() {
            let chapter_id = uuid::Uuid::new_v4().simple().to_string();
            let fingerprint = chapter_splitter.resolve_fingerprint(&raw_ch.title);
            let mut chapter_node =
                ChapterNode::new(chapter_id.clone(), c_idx + 1, fingerprint, raw_ch.title);

            let mut paragraph_node = ParagraphNode {
                id: format!("p_{chapter_id}"),
                index: 0,
                sentences: Vec::new(),
            };

            for (s_idx, line) in raw_ch.content.lines().filter(|l| !l.trim().is_empty()).enumerate() {
                let sentence_id = uuid::Uuid::new_v4().simple().to_string();
                let mut sentence_node = SentenceNode::new(sentence_id, s_idx, line.to_string());
                let raw_segments = dialogue_splitter.split_sentence(line);

                for raw_seg in raw_segments {
                    let seg_id = uuid::Uuid::new_v4().simple().to_string();
                    let seg_node = SegmentNode::new(
                        seg_id,
                        segment_global_index,
                        raw_seg.kind,
                        raw_seg.text,
                    );
                    sentence_node.segments.push(seg_node);
                    segment_global_index += 1;
                }
                paragraph_node.sentences.push(sentence_node);
            }

            chapter_node.paragraphs.push(paragraph_node);
            chapter_node.recalculate_word_count();
            chapter_nodes.push(chapter_node);
        }

        script.chapters = chapter_nodes;
        script.recalculate_totals();

        Ok(script)
    }
}
