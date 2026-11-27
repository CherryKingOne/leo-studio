//! 在 LLM 注册层里注册驱动 `minimax`，只依赖 `leo-core` 和共享协议库
//! `leo-llm-protocol`。支持 OpenAI Chat / OpenAI Responses / Anthropic 三种协议；
//! 只放行 MiniMax 自家 `MiniMax-M*` 文本模型。
//!
//! # 官方文档（维护时直接打开核对，不需要人工另外提供）
//! - API 概览：<https://platform.minimax.cn/docs/api-reference/api-overview>
//! - 模型介绍：<https://platform.minimax.cn/docs/guides/models-intro>
//! - 获取 API Key：<https://platform.minimax.cn/user-center>
//!
//! # 配置
//! API Key 只从环境变量 `MINIMAX_LLM_API_KEY` 读取（兼容 `MINIMAX_API_KEY`）；其余参数写在
//! `"settings": {"leo-plugin-llm-minimax": {...}}`，见 [`MinimaxConfig`]。
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

pub use client::Provider as MinimaxLlmProvider;
pub use config::Config as MinimaxConfig;
pub use models::builtin_models;

/// 插件 id
pub const PLUGIN_ID: &str = "leo-plugin-llm-minimax";
/// 在 LLM 注册表里的驱动 id
pub const DRIVER_ID: &str = "minimax";
/// UI / 日志里的展示名
pub const DISPLAY_NAME: &str = "MiniMax";

/// API Key 环境变量
pub const DEFAULT_API_KEY_ENV: &str = "MINIMAX_LLM_API_KEY";
/// 兼容的备用环境变量
pub const API_KEY_ENV_ALIASES: &[&str] = &["MINIMAX_API_KEY"];
/// 默认 OpenAI Chat / Responses 根地址；Anthropic 改成 `https://api.minimax.cn/anthropic`
pub const DEFAULT_BASE_URL: &str = "https://api.minimax.cn/v1";
/// 默认模型
pub const DEFAULT_MODEL: &str = "MiniMax-M3.1-Flash";
/// 远端模型列表；没有统一 `/models` 的厂商默认不填
pub const DEFAULT_MODELS_URL: Option<&str> = Some("https://api.minimax.cn/v1/models");
/// 本插件实际支持的协议
pub const SUPPORTED_PROTOCOLS: &[LlmProtocol] = &[
    LlmProtocol::OpenAiChat,
    LlmProtocol::OpenAiResponses,
    LlmProtocol::Anthropic,
];
pub const DEFAULT_THINKING_MODE: ThinkingMode = ThinkingMode::Default;
pub const DEFAULT_REASONING_EFFORT: Option<ReasoningEffort> = None;
/// 是否需要在 chat 请求体里注入 `thinking.type`（目前仅 Z.ai 需要）
pub const INSERTS_THINKING_OBJECT: bool = false;

pub struct MinimaxLlmPlugin {
    config: MinimaxConfig,
}

impl MinimaxLlmPlugin {
    pub fn new(config: MinimaxConfig) -> Self {
        Self { config }
    }

    pub fn from_settings(settings: Option<&serde_json::Value>) -> CoreResult<Self> {
        Ok(Self::new(MinimaxConfig::from_settings(settings)?))
    }
}

impl Default for MinimaxLlmPlugin {
    fn default() -> Self {
        Self::new(MinimaxConfig::default())
    }
}

#[async_trait]
impl Plugin for MinimaxLlmPlugin {
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
            "MiniMax 大语言模型",
            env!("CARGO_PKG_VERSION"),
            PluginCategory::Llm,
        )
        .with_description(
            "调用 MiniMax 的 Chat / Responses / Anthropic 兼容接口，注册为 LLM 驱动 minimax；只接入 MiniMax 自家文本模型，模型列表会过滤 Speech / Hailuo / Image / Music 等非对话模型。需要环境变量 MINIMAX_LLM_API_KEY（兼容 MINIMAX_API_KEY）。"
        )
        .with_capabilities(capabilities)
    }

    async fn apply(&self, ctx: &Context) -> CoreResult<()> {
        let Some(api_key) = self.config.api_key() else {
            tracing::warn!(
                "{DISPLAY_NAME} LLM 未注册：没有找到环境变量 {}（获取 Key：https://platform.minimax.cn/user-center）",
                self.config.api_key_env
            );
            return Ok(());
        };
        let provider: Arc<dyn LlmProvider> =
            Arc::new(MinimaxLlmProvider::new(self.config.clone(), api_key)?);
        ctx.register::<dyn LlmProvider>(DRIVER_ID, provider)?;
        tracing::info!(
            "{DISPLAY_NAME} LLM 插件加载完成，已注册驱动 {DRIVER_ID}（协议 {}，模型 {}）",
            self.config.protocol.as_str(),
            self.config.model
        );
        Ok(())
    }
}
