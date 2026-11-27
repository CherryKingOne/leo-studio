//! leo-plugin-llm-agnes：Agnes AI 大语言模型子插件。
//!
//! 在 LLM 注册层里注册驱动 `agnes`，只依赖 `leo-core` 和共享协议库
//! `leo-llm-protocol`。支持 OpenAI Chat / OpenAI Responses / Anthropic 三种协议，
//! 但只接受 Agnes 自家模型，模型列表会过滤掉混入的其他厂商模型。
//!
//! # 官方文档（维护时直接打开核对，不需要人工另外提供）
//! - 快速开始：<https://www.agnes-ai.com/zh-Hans/docs/quickstart>
//! - API 参考：<https://www.agnes-ai.com/zh-Hans/docs/api-reference>
//! - 获取 API Key：<https://www.agnes-ai.com/zh-Hans/docs/quickstart>
//!
//! # 配置
//! API Key 只从环境变量 `AGNES_API_KEY` 读取；其余参数写在
//! `"settings": {"leo-plugin-llm-agnes": {...}}`，见 [`AgnesConfig`]。
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

pub use client::Provider as AgnesLlmProvider;
pub use config::Config as AgnesConfig;
pub use models::builtin_models;

/// 插件 id
pub const PLUGIN_ID: &str = "leo-plugin-llm-agnes";
/// 在 LLM 注册表里的驱动 id
pub const DRIVER_ID: &str = "agnes";
/// UI / 日志里的展示名
pub const DISPLAY_NAME: &str = "Agnes AI";

/// API Key 环境变量
pub const DEFAULT_API_KEY_ENV: &str = "AGNES_API_KEY";
/// 兼容的备用环境变量
pub const API_KEY_ENV_ALIASES: &[&str] = &[];
/// 默认 OpenAI Chat / Responses 根地址；切 Anthropic 时改成 `https://apihub.agnes-ai.com`
pub const DEFAULT_BASE_URL: &str = "https://apihub.agnes-ai.com/v1";
/// 默认模型
pub const DEFAULT_MODEL: &str = "agnes-3.0-flash";
/// 远端模型列表；Agnes 有统一 `/models`
pub const DEFAULT_MODELS_URL: Option<&str> = Some("https://apihub.agnes-ai.com/v1/models");
/// 本插件实际支持的协议
pub const SUPPORTED_PROTOCOLS: &[LlmProtocol] = &[
    LlmProtocol::OpenAiChat,
    LlmProtocol::OpenAiResponses,
    LlmProtocol::Anthropic,
];
pub const DEFAULT_THINKING_MODE: ThinkingMode = ThinkingMode::Default;
pub const DEFAULT_REASONING_EFFORT: Option<ReasoningEffort> = None;
/// Agnes 的思考开关走协议标准字段，不需要额外注入 thinking 对象
pub const INSERTS_THINKING_OBJECT: bool = false;

pub struct AgnesLlmPlugin {
    config: AgnesConfig,
}

impl AgnesLlmPlugin {
    pub fn new(config: AgnesConfig) -> Self {
        Self { config }
    }

    pub fn from_settings(settings: Option<&serde_json::Value>) -> CoreResult<Self> {
        Ok(Self::new(AgnesConfig::from_settings(settings)?))
    }
}

impl Default for AgnesLlmPlugin {
    fn default() -> Self {
        Self::new(AgnesConfig::default())
    }
}

#[async_trait]
impl Plugin for AgnesLlmPlugin {
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
            "Agnes AI 大语言模型",
            env!("CARGO_PKG_VERSION"),
            PluginCategory::Llm,
        )
        .with_description(format!(
            "调用 Agnes AI 的 Chat / Responses / Anthropic 兼容接口，注册为 LLM 驱动 {DRIVER_ID}；只接入 Agnes 自家模型，支持获取模型列表。需要环境变量 {DEFAULT_API_KEY_ENV}。"
        ))
        .with_capabilities(capabilities)
    }

    async fn apply(&self, ctx: &Context) -> CoreResult<()> {
        let Some(api_key) = self.config.api_key() else {
            tracing::warn!(
                "{DISPLAY_NAME} LLM 未注册：没有找到环境变量 {}（获取 Key：https://www.agnes-ai.com/zh-Hans/docs/quickstart）",
                self.config.api_key_env
            );
            return Ok(());
        };
        let provider: Arc<dyn LlmProvider> =
            Arc::new(AgnesLlmProvider::new(self.config.clone(), api_key)?);
        ctx.register::<dyn LlmProvider>(DRIVER_ID, provider)?;
        tracing::info!(
            "{DISPLAY_NAME} LLM 插件加载完成，已注册驱动 {DRIVER_ID}（协议 {}，模型 {}）",
            self.config.protocol.as_str(),
            self.config.model
        );
        Ok(())
    }
}
