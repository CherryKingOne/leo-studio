//! 在 LLM 注册层里注册驱动 `gemini`，只依赖 `leo-core` 和共享协议库
//! `leo-llm-protocol`。Gemini 原生协议不属于本项目的三种协议，本插件走官方
//! OpenAI 兼容层，因此只声明 OpenAI Chat；只放行 Gemini 自家 `gemini-*` 文本模型。
//!
//! # 官方文档（维护时直接打开核对，不需要人工另外提供）
//! - API 文档：<https://aistudio.google.com/docs>
//! - API Reference：<https://aistudio.google.com/api-reference>
//! - 模型列表：<https://ai.google.dev/gemini-api/docs/models>
//! - 获取 API Key：<https://aistudio.google.com/apikey>
//!
//! # 配置
//! API Key 只从环境变量 `GEMINI_API_KEY` 读取；其余参数写在
//! `"settings": {"leo-plugin-llm-gemini": {...}}`，见 [`GeminiConfig`]。
//! 没有 Key 时插件照常装配但不注册驱动，不影响其他插件。

mod client;
mod config;
mod models;

use std::sync::Arc;

use async_trait::async_trait;
use leo_core::context::Context;
use leo_core::error::CoreResult;
use leo_core::plugin::{Plugin, PluginCategory, PluginManifest};
use leo_core::spi::{LlmProtocol, LlmProvider, ReasoningEffort, ThinkingMode};

pub use client::Provider as GeminiLlmProvider;
pub use config::Config as GeminiConfig;
pub use models::builtin_models;

/// 插件 id
pub const PLUGIN_ID: &str = "leo-plugin-llm-gemini";
/// 在 LLM 注册表里的驱动 id
pub const DRIVER_ID: &str = "gemini";
/// UI / 日志里的展示名
pub const DISPLAY_NAME: &str = "Google Gemini";

/// API Key 环境变量
pub const DEFAULT_API_KEY_ENV: &str = "GEMINI_API_KEY";
/// 兼容的备用环境变量
pub const API_KEY_ENV_ALIASES: &[&str] = &["GOOGLE_API_KEY", "GOOGLE_GENERATIVE_AI_API_KEY"];
/// Gemini 的 OpenAI 兼容层根地址；只支持 OpenAI Chat
pub const DEFAULT_BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta/openai";
/// 默认模型
pub const DEFAULT_MODEL: &str = "gemini-4-hydrogen";
/// 远端模型列表；没有统一 `/models` 的厂商默认不填
pub const DEFAULT_MODELS_URL: Option<&str> =
    Some("https://generativelanguage.googleapis.com/v1beta/openai/models");
/// 本插件实际支持的协议
pub const SUPPORTED_PROTOCOLS: &[LlmProtocol] = &[LlmProtocol::OpenAiChat];
pub const DEFAULT_THINKING_MODE: ThinkingMode = ThinkingMode::Default;
pub const DEFAULT_REASONING_EFFORT: Option<ReasoningEffort> = None;
/// 是否需要在 chat 请求体里注入 `thinking.type`（目前仅 Z.ai 需要）
pub const INSERTS_THINKING_OBJECT: bool = false;

pub struct GeminiLlmPlugin {
    config: GeminiConfig,
}

impl GeminiLlmPlugin {
    pub fn new(config: GeminiConfig) -> Self {
        Self { config }
    }

    pub fn from_settings(settings: Option<&serde_json::Value>) -> CoreResult<Self> {
        Ok(Self::new(GeminiConfig::from_settings(settings)?))
    }
}

impl Default for GeminiLlmPlugin {
    fn default() -> Self {
        Self::new(GeminiConfig::default())
    }
}

#[async_trait]
impl Plugin for GeminiLlmPlugin {
    fn manifest(&self) -> PluginManifest {
        let protocols = SUPPORTED_PROTOCOLS
            .iter()
            .map(|protocol| format!("llm:protocol:{}", protocol.as_str()));
        let capabilities = ["llm:chat", "llm:list_models"]
            .into_iter()
            .map(str::to_string)
            .chain(protocols);
        PluginManifest::new(
            PLUGIN_ID,
            "Google Gemini 大语言模型",
            env!("CARGO_PKG_VERSION"),
            PluginCategory::Llm,
        )
        .with_description(
            "调用 Gemini 的 OpenAI 兼容接口，注册为 LLM 驱动 gemini；只接入 Gemini 自家文本模型，模型列表会过滤 TTS / Live / Image / Embedding / Omni 等非对话模型。需要环境变量 GEMINI_API_KEY。"
        )
        .with_capabilities(capabilities)
    }

    async fn apply(&self, ctx: &Context) -> CoreResult<()> {
        let Some(api_key) = self.config.api_key() else {
            tracing::warn!(
                "{DISPLAY_NAME} LLM 未注册：没有找到环境变量 {}（获取 Key：https://aistudio.google.com/apikey）",
                self.config.api_key_env
            );
            return Ok(());
        };
        let provider: Arc<dyn LlmProvider> =
            Arc::new(GeminiLlmProvider::new(self.config.clone(), api_key)?);
        ctx.register::<dyn LlmProvider>(DRIVER_ID, provider)?;
        tracing::info!(
            "{DISPLAY_NAME} LLM 插件加载完成，已注册驱动 {DRIVER_ID}（协议 {}，模型 {}）",
            self.config.protocol.as_str(),
            self.config.model
        );
        Ok(())
    }
}
