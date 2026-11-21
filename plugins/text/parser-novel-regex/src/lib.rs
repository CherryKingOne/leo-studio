pub mod algorithm;

use std::sync::Arc;

use async_trait::async_trait;
use leo_core::context::Context;
use leo_core::error::CoreResult;
use leo_core::plugin::{ChapterSplitter, Plugin, PluginCategory, PluginManifest, RawChapter};

pub use algorithm::{
    chapter_number, clean_text, cn_to_int, count_words, parse_chapters, parse_novel_filename,
    parse_pinyin_elements, resolve_chapter_key,
};

/// 本插件在注册表里的驱动 id
pub const DRIVER_ID: &str = "novel-regex";

pub struct NovelRegexParserPlugin;

impl Default for NovelRegexParserPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl NovelRegexParserPlugin {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Plugin for NovelRegexParserPlugin {
    fn manifest(&self) -> PluginManifest {
        let mut manifest = PluginManifest::new(
            "leo-plugin-parser-novel-regex",
            "中文网络小说正则分章插件",
            "0.1.0",
            PluginCategory::Parser,
        );
        manifest.description =
            "支持第X章/回/节/卷正则分章、中文大写数字转阿拉伯数字、以及防脏数据章节指纹比对。".into();
        manifest.capabilities = vec!["parser:split_chapters".into(), "parser:chapter_key".into()];
        manifest
    }

    async fn apply(&self, ctx: &Context) -> CoreResult<()> {
        let splitter: Arc<dyn ChapterSplitter> = Arc::new(NovelRegexSplitter);
        ctx.register::<dyn ChapterSplitter>(DRIVER_ID, splitter)?;
        tracing::info!("已挂载 NovelRegexParserPlugin，已注册分章器 novel-regex");
        Ok(())
    }
}

pub struct NovelRegexSplitter;

impl ChapterSplitter for NovelRegexSplitter {
    fn split_chapters(&self, full_text: &str) -> Vec<RawChapter> {
        parse_chapters(full_text)
            .into_iter()
            .map(|(title, content)| RawChapter { title, content })
            .collect()
    }

    fn resolve_fingerprint(&self, title: &str) -> String {
        resolve_chapter_key(title)
    }

    fn parse_filename(&self, filename: &str) -> (String, String, String) {
        parse_novel_filename(filename)
    }
}
