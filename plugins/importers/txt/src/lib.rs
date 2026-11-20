use std::sync::Arc;

use async_trait::async_trait;
use leo_core::ast::{ChapterNode, ParagraphNode, ScriptDocument, SegmentNode, SentenceNode};
use leo_core::context::Context;
use leo_core::error::{CoreError, CoreResult};
use leo_core::plugin::{
    ChapterSplitter, DialogueSplitter, Importer, Plugin, PluginCategory, PluginManifest, TextCleaner,
};
use leo_plugin_cleaner_kaomoji::KaomojiCleanerProvider;
use leo_plugin_dialogue_quotes::QuotesDialogueSplitter;
use leo_plugin_parser_novel_regex::{NovelRegexSplitter, parse_novel_filename};

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
        let mut manifest = PluginManifest::new(
            "leo-plugin-importer-txt",
            "TXT 纯文本小说导入插件",
            "0.1.0",
            PluginCategory::Importer,
        );
        manifest.description =
            "支持 .txt 格式纯文本小说解析，自动执行编码解码、标题识别、清洗分章并生成 Script AST。".into();
        manifest.capabilities = vec!["importer:txt".into()];
        manifest.dependencies = vec![
            "leo-plugin-cleaner-kaomoji".into(),
            "leo-plugin-parser-novel-regex".into(),
            "leo-plugin-dialogue-quotes".into(),
        ];
        manifest
    }

    async fn apply(&self, ctx: &Context) -> CoreResult<()> {
        let importer: Arc<dyn Importer> = Arc::new(TxtImporter::with_services(
            ctx.inject::<Arc<dyn TextCleaner>>().await?.as_ref().clone(),
            ctx.inject::<Arc<dyn ChapterSplitter>>().await?.as_ref().clone(),
            ctx.inject::<Arc<dyn DialogueSplitter>>().await?.as_ref().clone(),
        ));
        ctx.provide_named("txt", importer).await?;
        tracing::info!("已挂载 TxtImporterPlugin，提供 Importer 服务");
        Ok(())
    }
}

pub struct TxtImporter {
    cleaner: Arc<dyn TextCleaner>,
    chapter_splitter: Arc<dyn ChapterSplitter>,
    dialogue_splitter: Arc<dyn DialogueSplitter>,
}

impl Default for TxtImporter {
    fn default() -> Self {
        Self::new()
    }
}

impl TxtImporter {
    /// 使用官方默认的清洗、分章、台词切分实现（脱离插件框架单独使用时）
    pub fn new() -> Self {
        Self::with_services(
            Arc::new(KaomojiCleanerProvider),
            Arc::new(NovelRegexSplitter),
            Arc::new(QuotesDialogueSplitter),
        )
    }

    /// 使用 Context 中挂载的文本处理服务
    pub fn with_services(
        cleaner: Arc<dyn TextCleaner>,
        chapter_splitter: Arc<dyn ChapterSplitter>,
        dialogue_splitter: Arc<dyn DialogueSplitter>,
    ) -> Self {
        Self {
            cleaner,
            chapter_splitter,
            dialogue_splitter,
        }
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

        let raw_text = Self::decode_bytes(content);
        let (parsed_title, author, genre) = parse_novel_filename(filename);

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
            .map(|l| self.cleaner.clean_line(l))
            .filter(|l| !l.is_empty())
            .collect();
        let cleaned_text = cleaned_lines.join("\n");

        // 2. 章节切分
        let raw_chapters = self.chapter_splitter.split_chapters(&cleaned_text);
        let mut chapter_nodes = Vec::new();

        let mut segment_global_index = 0;
        for (c_idx, raw_ch) in raw_chapters.into_iter().enumerate() {
            let chapter_id = uuid::Uuid::new_v4().simple().to_string();
            let fingerprint = self.chapter_splitter.resolve_fingerprint(&raw_ch.title);
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
                let raw_segments = self.dialogue_splitter.split_sentence(line);

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
