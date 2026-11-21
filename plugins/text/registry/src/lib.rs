//! leo-plugin-text：文本处理**注册层**插件。
//!
//! 三个扩展点各有一张注册表：
//! - `TextCleaner`：可以注册多个，按配置顺序串成**清洗链**（不配置时按注册顺序全部启用）；
//! - `ChapterSplitter`：分章器，按配置选一个（默认最早注册的）；
//! - `DialogueSplitter`：台词切分器，按配置选一个。
//!
//! 导入器、接口层只依赖本插件的 [`TextHub`]，不直接依赖任何具体的清洗 / 分章插件。
//!
//! 配置：
//! ```json
//! { "cleaners": ["kaomoji"], "chapter_splitter": "novel-regex", "dialogue_splitter": "quotes" }
//! ```

use std::sync::Arc;

use async_trait::async_trait;
use leo_core::context::Context;
use leo_core::error::CoreResult;
use leo_core::plugin::{Plugin, PluginCategory, PluginManifest};
use leo_core::registry::{DriverInfo, Registry};
use leo_core::spi::{ChapterSplitter, DialogueSplitter, TextCleaner};
use serde::{Deserialize, Serialize};

pub const PLUGIN_ID: &str = "leo-plugin-text";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TextRegistryConfig {
    /// 清洗链：按顺序执行的清洗器 id；`None` 表示全部已注册的清洗器按注册顺序执行，`[]` 表示不清洗
    pub cleaners: Option<Vec<String>>,
    pub chapter_splitter: Option<String>,
    pub dialogue_splitter: Option<String>,
}

#[derive(Default)]
pub struct TextRegistryPlugin {
    config: TextRegistryConfig,
}

impl TextRegistryPlugin {
    pub fn new(config: TextRegistryConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl Plugin for TextRegistryPlugin {
    fn manifest(&self) -> PluginManifest {
        PluginManifest::new(PLUGIN_ID, "文本处理注册层", env!("CARGO_PKG_VERSION"), PluginCategory::Registry)
            .with_description("管理清洗 / 分章 / 台词切分子插件，组装清洗链并挑选分章器与台词切分器。")
            .with_capabilities(["registry:text-cleaner", "registry:chapter-splitter", "registry:dialogue-splitter"])
    }

    async fn apply(&self, ctx: &Context) -> CoreResult<()> {
        let hub = TextHub::from_registries(ctx, self.config.cleaners.clone());
        hub.chapters.set_default(self.config.chapter_splitter.as_deref());
        hub.dialogues.set_default(self.config.dialogue_splitter.as_deref());
        let (c, d) = (hub.chapters.clone(), hub.dialogues.clone());
        ctx.on_dispose(move || {
            c.set_default(None);
            d.set_default(None);
        });
        ctx.provide(hub).await;
        Ok(())
    }
}

#[derive(Clone)]
pub struct TextHub {
    cleaners: Registry<dyn TextCleaner>,
    chapters: Registry<dyn ChapterSplitter>,
    dialogues: Registry<dyn DialogueSplitter>,
    cleaner_order: Option<Vec<String>>,
}

impl TextHub {
    fn from_registries(ctx: &Context, cleaner_order: Option<Vec<String>>) -> Self {
        Self {
            cleaners: ctx.registry::<dyn TextCleaner>(),
            chapters: ctx.registry::<dyn ChapterSplitter>(),
            dialogues: ctx.registry::<dyn DialogueSplitter>(),
            cleaner_order,
        }
    }

    pub async fn of(ctx: &Context) -> Self {
        match ctx.inject::<TextHub>().await {
            Ok(hub) => (*hub).clone(),
            Err(_) => Self::from_registries(ctx, None),
        }
    }

    pub fn cleaner_drivers(&self) -> Vec<DriverInfo> {
        self.cleaners.describe()
    }
    pub fn chapter_drivers(&self) -> Vec<DriverInfo> {
        self.chapters.describe()
    }
    pub fn dialogue_drivers(&self) -> Vec<DriverInfo> {
        self.dialogues.describe()
    }

    /// 当前清洗链（配置里写了但没注册的清洗器会被跳过并警告）
    pub fn cleaner_chain(&self) -> Vec<(String, Arc<dyn TextCleaner>)> {
        match &self.cleaner_order {
            None => self.cleaners.entries(),
            Some(order) => order
                .iter()
                .filter_map(|id| match self.cleaners.get(id) {
                    Some(c) => Some((id.clone(), c)),
                    None => {
                        tracing::warn!("清洗链里的清洗器 {id} 没有注册，已跳过");
                        None
                    }
                })
                .collect(),
        }
    }

    /// 让一行文本依次通过清洗链
    pub fn clean_line(&self, line: &str) -> String {
        self.cleaner_chain().iter().fold(line.to_string(), |acc, (_, c)| c.clean_line(&acc))
    }

    pub fn chapter_splitter(&self) -> CoreResult<Arc<dyn ChapterSplitter>> {
        self.chapters.resolve(None).map(|(_, s)| s)
    }

    pub fn dialogue_splitter(&self) -> CoreResult<Arc<dyn DialogueSplitter>> {
        self.dialogues.resolve(None).map(|(_, s)| s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Upper;
    impl TextCleaner for Upper {
        fn clean_line(&self, t: &str) -> String {
            t.to_uppercase()
        }
    }
    struct Trim;
    impl TextCleaner for Trim {
        fn clean_line(&self, t: &str) -> String {
            t.trim().to_string()
        }
    }

    #[tokio::test]
    async fn cleaner_chain_follows_config() {
        let root = Context::new("t");
        root.register::<dyn TextCleaner>("upper", Arc::new(Upper)).unwrap();
        root.register::<dyn TextCleaner>("trim", Arc::new(Trim)).unwrap();
        assert_eq!(TextHub::of(&root).await.clean_line(" ab "), "AB");

        let p = root.for_plugin(PLUGIN_ID);
        TextRegistryPlugin::new(TextRegistryConfig { cleaners: Some(vec!["trim".into(), "missing".into()]), ..Default::default() })
            .apply(&p)
            .await
            .unwrap();
        assert_eq!(TextHub::of(&root).await.clean_line(" ab "), "ab");
        assert!(TextHub::of(&root).await.chapter_splitter().is_err());
    }
}
