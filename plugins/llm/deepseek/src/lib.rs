//! leo-plugin-llm-deepseek：DeepSeek 大模型子插件（“小盒子”）。
//!
//! 在 LLM 注册层（leo-plugin-llm）里注册驱动 id [`DRIVER_ID`]（`deepseek`），只依赖 `leo-core`。
//! 用法：在 `leo-plugins.json` 把 `"llm": {"default": "deepseek"}` 设成默认驱动，或 `LlmHub::chat_with("deepseek", ..)`。
//!
//! # 官方文档（维护时直接打开这些链接核对接口）
//! - 文档首页：<https://api-docs.deepseek.com>
//! - Chat Completions `POST /chat/completions`：<https://api-docs.deepseek.com/api/create-chat-completion>
//! - 模型与价格（当前模型名）：<https://api-docs.deepseek.com/quick_start/pricing>
//! - JSON 输出：<https://api-docs.deepseek.com/guides/json_mode>
//! - 思考模式开关：<https://api-docs.deepseek.com/guides/thinking_mode>
//! - 错误码：<https://api-docs.deepseek.com/quick_start/error_codes>
//! - 更新日志（模型改名 / 下线）：<https://api-docs.deepseek.com/updates>
//! - 获取 API Key：<https://platform.deepseek.com/api_keys>
//!
//! # 配置
//! API Key 只从环境变量读取（默认 `DEEPSEEK_API_KEY`），不写进配置文件。其余参数写在
//! `leo-plugins.json` 的 `"settings": {"leo-plugin-llm-deepseek": {...}}` 里，见 [`DeepSeekConfig`]。
//! 没有 API Key 时驱动照常注册，调用时返回明确的错误（不会悄悄换成别的驱动）。

mod client;
mod config;

use std::sync::Arc;

use async_trait::async_trait;
use leo_core::context::Context;
use leo_core::error::CoreResult;
use leo_core::plugin::{Plugin, PluginCategory, PluginManifest};
use leo_core::spi::LlmProvider;

pub use client::{DeepSeekLlmProvider, is_retryable, map_http_error, parse_response};
pub use config::DeepSeekConfig;

/// 插件 id
pub const PLUGIN_ID: &str = "leo-plugin-llm-deepseek";
/// 在 LLM 注册表里的驱动 id
pub const DRIVER_ID: &str = "deepseek";

pub struct DeepSeekLlmPlugin {
    config: DeepSeekConfig,
}

impl DeepSeekLlmPlugin {
    pub fn new(config: DeepSeekConfig) -> Self {
        Self { config }
    }

    /// 从 `leo-plugins.json` 的 settings 一节解析配置（没有这一节时用默认配置）
    pub fn from_settings(settings: Option<&serde_json::Value>) -> CoreResult<Self> {
        Ok(Self::new(DeepSeekConfig::from_settings(settings)?))
    }
}

impl Default for DeepSeekLlmPlugin {
    fn default() -> Self {
        Self::new(DeepSeekConfig::default())
    }
}

#[async_trait]
impl Plugin for DeepSeekLlmPlugin {
    fn manifest(&self) -> PluginManifest {
        PluginManifest::new(PLUGIN_ID, "DeepSeek 大模型", env!("CARGO_PKG_VERSION"), PluginCategory::Llm)
            .with_description("调用 DeepSeek /chat/completions（OpenAI 兼容），注册为 LLM 驱动 deepseek。需要环境变量 DEEPSEEK_API_KEY。")
            .with_capabilities(["llm:chat", "llm:json"])
    }

    async fn apply(&self, ctx: &Context) -> CoreResult<()> {
        if self.config.api_key().is_none() {
            tracing::warn!(
                "DeepSeek LLM 已注册，但没有找到环境变量 {}，调用时会报错（获取 Key：https://platform.deepseek.com/api_keys）",
                self.config.api_key_env
            );
        }
        let provider: Arc<dyn LlmProvider> = Arc::new(DeepSeekLlmProvider::new(self.config.clone())?);
        ctx.register::<dyn LlmProvider>(DRIVER_ID, provider)?;
        tracing::info!("DeepSeek LLM 插件加载完成，已注册 LLM 驱动 {DRIVER_ID}（模型 {}）", self.config.model);
        Ok(())
    }
}
