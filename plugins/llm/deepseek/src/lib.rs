//! leo-plugin-llm-deepseek：DeepSeek 大语言模型子插件（“小盒子”）。
//!
//! 在 LLM 注册层（leo-plugin-llm）里注册驱动 id [`DRIVER_ID`]（`deepseek`），只依赖
//! `leo-core` 和共享协议库 `leo-llm-protocol`。
//!
//! 用法：模型写 `deepseek:deepseek-chat`；或在 `leo-plugins.json` 把
//! `"llm": {"default": "deepseek"}` 设成默认驱动。
//!
//! # 官方文档（维护时直接打开这些链接核对接口，不需要人工另外提供）
//! - 文档首页：<https://api-docs.deepseek.com/>
//! - Chat Completions（OpenAI 旧接口）：<https://api-docs.deepseek.com/api/create-chat-completion>
//! - Anthropic 兼容接口说明：<https://api-docs.deepseek.com/guides/anthropic_api>
//! - 模型与上下文长度：<https://api-docs.deepseek.com/quick_start/pricing>
//! - 获取 API Key：<https://platform.deepseek.com/api_keys>
//!
//! # 配置
//! API Key 只从环境变量读取（默认 `DEEPSEEK_API_KEY`），不写进配置文件。其余参数写在
//! `leo-plugins.json` 的 `"settings": {"leo-plugin-llm-deepseek": {...}}` 里，见 [`DeepseekConfig`]。
//! 没有 API Key 时插件照常装配但不注册驱动（只打一条警告），不影响其他插件。

mod client;
mod config;
mod models;

use std::sync::Arc;

use async_trait::async_trait;
use leo_core::context::Context;
use leo_core::error::CoreResult;
use leo_core::plugin::{Plugin, PluginCategory, PluginManifest};
use leo_core::spi::LlmProvider;

pub use client::DeepseekLlmProvider;
pub use config::DeepseekConfig;
pub use models::builtin_models;

/// 插件 id
pub const PLUGIN_ID: &str = "leo-plugin-llm-deepseek";
/// 在 LLM 注册表里的驱动 id
pub const DRIVER_ID: &str = "deepseek";

pub struct DeepseekLlmPlugin {
    config: DeepseekConfig,
}

impl DeepseekLlmPlugin {
    pub fn new(config: DeepseekConfig) -> Self {
        Self { config }
    }

    /// 从 `leo-plugins.json` 的 settings 一节解析配置（没有这一节时用默认配置）
    pub fn from_settings(settings: Option<&serde_json::Value>) -> CoreResult<Self> {
        Ok(Self::new(DeepseekConfig::from_settings(settings)?))
    }
}

impl Default for DeepseekLlmPlugin {
    fn default() -> Self {
        Self::new(DeepseekConfig::default())
    }
}

#[async_trait]
impl Plugin for DeepseekLlmPlugin {
    fn manifest(&self) -> PluginManifest {
        PluginManifest::new(PLUGIN_ID, "DeepSeek 大语言模型", env!("CARGO_PKG_VERSION"), PluginCategory::Llm)
            .with_description(
                "调用 DeepSeek 的 Chat Completions / Responses / Anthropic 兼容接口，注册为 LLM 驱动 deepseek。需要环境变量 DEEPSEEK_API_KEY。",
            )
            .with_capabilities(["llm:chat", "llm:list_models", "llm:protocol:openai-chat", "llm:protocol:openai-responses", "llm:protocol:anthropic"])
    }

    async fn apply(&self, ctx: &Context) -> CoreResult<()> {
        let Some(api_key) = self.config.api_key() else {
            tracing::warn!(
                "DeepSeek LLM 未注册：没有找到环境变量 {}（获取 Key：https://platform.deepseek.com/api_keys）",
                self.config.api_key_env
            );
            return Ok(());
        };
        let provider: Arc<dyn LlmProvider> =
            Arc::new(DeepseekLlmProvider::new(self.config.clone(), api_key)?);
        ctx.register::<dyn LlmProvider>(DRIVER_ID, provider)?;
        tracing::info!(
            "DeepSeek LLM 插件加载完成，已注册 LLM 驱动 {DRIVER_ID}（协议 {}，模型 {}）",
            self.config.protocol.as_str(),
            self.config.model
        );
        Ok(())
    }
}
