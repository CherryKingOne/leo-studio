pub mod processor;

use std::sync::Arc;

use leo_core::context::Context;
use leo_core::error::CoreResult;
use leo_core::plugin::{ChapterSplitter, DialogueSplitter, RawDialogueLine, TextCleaner};

pub use leo_plugin_parser_novel_regex::{count_words, parse_novel_filename, parse_pinyin_elements};
pub use processor::{
    build_sentence_records, clean_dialogue_records, create_dialogue_records_from_sentence,
    create_sentence_records,
};

/// REST / MCP 项目接口使用的文本处理管线，全部能力来自 Context 中挂载的插件服务。
#[derive(Clone)]
pub struct TextPipeline {
    pub cleaner: Arc<dyn TextCleaner>,
    pub chapters: Arc<dyn ChapterSplitter>,
    pub dialogue: Arc<dyn DialogueSplitter>,
}

impl TextPipeline {
    pub fn new(
        cleaner: Arc<dyn TextCleaner>,
        chapters: Arc<dyn ChapterSplitter>,
        dialogue: Arc<dyn DialogueSplitter>,
    ) -> Self {
        Self {
            cleaner,
            chapters,
            dialogue,
        }
    }

    /// 从 Context 取出默认的清洗、分章与台词切分服务
    pub async fn from_context(ctx: &Context) -> CoreResult<Self> {
        Ok(Self {
            cleaner: ctx.inject::<Arc<dyn TextCleaner>>().await?.as_ref().clone(),
            chapters: ctx.inject::<Arc<dyn ChapterSplitter>>().await?.as_ref().clone(),
            dialogue: ctx.inject::<Arc<dyn DialogueSplitter>>().await?.as_ref().clone(),
        })
    }

    pub fn clean_line(&self, line: &str) -> String {
        self.cleaner.clean_line(line)
    }

    /// 逐行清洗全文，丢弃清洗后为空的行。
    pub fn clean_text(&self, text: &str) -> String {
        text.lines()
            .map(|line| self.cleaner.clean_line(line))
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// 对已清洗文本分章，返回 (标题, 正文)。
    pub fn parse_chapters(&self, cleaned: &str) -> Vec<(String, String)> {
        self.chapters
            .split_chapters(cleaned)
            .into_iter()
            .map(|chapter| (chapter.title, chapter.content))
            .collect()
    }

    pub fn chapter_key(&self, title: &str) -> String {
        self.chapters.resolve_fingerprint(title)
    }

    pub fn chapter_number(&self, title: &str) -> Option<i64> {
        self.chapters.chapter_number(title)
    }

    /// 将章节正文按行切分为句子列表（逐行清洗，丢弃空行）。
    pub fn split_sentences(&self, content: &str) -> Vec<String> {
        content
            .lines()
            .map(|line| self.cleaner.clean_line(line))
            .filter(|line| !line.is_empty())
            .collect()
    }

    pub fn split_dialogue(&self, sentence: &str) -> RawDialogueLine {
        self.dialogue.analyze_sentence(sentence)
    }
}
