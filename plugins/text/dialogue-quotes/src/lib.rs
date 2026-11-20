pub mod algorithm;

use std::sync::Arc;

use async_trait::async_trait;
use leo_core::ast::SegmentKind;
use leo_core::context::Context;
use leo_core::error::CoreResult;
use leo_core::plugin::{
    DialogueSplitter, Plugin, PluginCategory, PluginManifest, RawDialogueSegment,
};

pub use algorithm::{DialogueLine, DialogueSegment, ParseStatus, SegmentType, split_dialogue_line};

pub struct DialogueQuotesPlugin;

impl Default for DialogueQuotesPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl DialogueQuotesPlugin {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Plugin for DialogueQuotesPlugin {
    fn manifest(&self) -> PluginManifest {
        let mut manifest = PluginManifest::new(
            "leo-plugin-dialogue-quotes",
            "中文双引号台词/旁白切分插件",
            "0.1.0",
            PluginCategory::Dialogue,
        );
        manifest.description =
            "基于成对中文双引号“”识别小说中的角色对白台词与叙述旁白，并标记悬空引号。".into();
        manifest.capabilities = vec!["dialogue:quotes".into()];
        manifest
    }

    async fn apply(&self, ctx: &Context) -> CoreResult<()> {
        let splitter: Arc<dyn DialogueSplitter> = Arc::new(QuotesDialogueSplitter);
        ctx.provide_named("quotes", splitter).await?;
        tracing::info!("已挂载 DialogueQuotesPlugin，提供 DialogueSplitter 服务");
        Ok(())
    }
}

pub struct QuotesDialogueSplitter;

impl DialogueSplitter for QuotesDialogueSplitter {
    fn split_sentence(&self, sentence: &str) -> Vec<RawDialogueSegment> {
        let line = split_dialogue_line(sentence);
        line.segments
            .into_iter()
            .map(|seg| RawDialogueSegment {
                text: seg.content,
                kind: match seg.segment_type {
                    SegmentType::Narration => SegmentKind::Narration,
                    SegmentType::Dialogue => SegmentKind::Dialogue,
                },
            })
            .collect()
    }
}
