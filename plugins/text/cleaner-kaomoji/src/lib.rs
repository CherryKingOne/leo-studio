pub mod algorithm;

use std::sync::Arc;

use async_trait::async_trait;
use leo_core::context::Context;
use leo_core::error::CoreResult;
use leo_core::plugin::{Plugin, PluginCategory, PluginManifest, TextCleaner};

pub use algorithm::{clean_line_text, is_han, strip_kaomoji};

/// 本插件在注册表里的驱动 id
pub const DRIVER_ID: &str = "kaomoji";

pub struct KaomojiCleanerPlugin;

impl Default for KaomojiCleanerPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl KaomojiCleanerPlugin {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Plugin for KaomojiCleanerPlugin {
    fn manifest(&self) -> PluginManifest {
        let mut manifest = PluginManifest::new(
            "leo-plugin-cleaner-kaomoji",
            "颜文字与异常符号清洗插件",
            "0.1.0",
            PluginCategory::Cleaner,
        );
        manifest.description =
            "基于 60+ 组 Unicode 范围与结构特征，过滤小说中的各种花式颜文字与广告异常符号。".into();
        manifest.capabilities = vec!["cleaner:text".into()];
        manifest
    }

    async fn apply(&self, ctx: &Context) -> CoreResult<()> {
        let cleaner: Arc<dyn TextCleaner> = Arc::new(KaomojiCleanerProvider);
        ctx.register::<dyn TextCleaner>(DRIVER_ID, cleaner)?;
        tracing::info!("已挂载 KaomojiCleanerPlugin，已注册清洗器 kaomoji");
        Ok(())
    }
}

pub struct KaomojiCleanerProvider;

impl TextCleaner for KaomojiCleanerProvider {
    fn clean_line(&self, text: &str) -> String {
        clean_line_text(text)
    }
}
