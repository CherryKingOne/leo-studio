//! 在 LLM 注册层里注册驱动 `zai`，只依赖 `leo-core` 和共享协议库
//! `leo-llm-protocol`。支持 OpenAI Chat 与 Anthropic 两种协议（中国站公开文档
//! 没有 Responses 入口）；只放行智谱自家 `glm-*` 模型，并把思考开关写进
//! `thinking.type`。
//!
//! # 官方文档（维护时直接打开核对，不需要人工另外提供）
//! - API 介绍：<https://docs.bigmodel.cn/cn/api/introduction>
//! - 快速开始：<https://docs.bigmodel.cn/cn/guide/start/introduction>
//! - 模型列表：<https://docs.bigmodel.cn/cn/guide/models/text/glm-5>
//! - 获取 API Key：<https://bigmodel.cn/usercenter/proj-mgmt/apikeys>
//!
//! # 配置
//! API Key 只从环境变量 `ZAI_API_KEY` 读取；其余参数写在
//! `"settings": {"leo-plugin-llm-zai": {...}}`，见 [`ZaiConfig`]。
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

pub use client::Provider as ZaiLlmProvider;
pub use config::Config as ZaiConfig;
pub use models::builtin_models;

/// 插件 id
pub const PLUGIN_ID: &str = "leo-plugin-llm-zai";
/// 在 LLM 注册表里的驱动 id
pub const DRIVER_ID: &str = "zai";
/// UI / 日志里的展示名
pub const DISPLAY_NAME: &str = "智谱 Z.ai";

/// API Key 环境变量
pub const DEFAULT_API_KEY_ENV: &str = "ZAI_API_KEY";
/// 兼容的备用环境变量
pub const API_KEY_ENV_ALIASES: &[&str] = &["ZHIPU_API_KEY", "ZHIPUAI_API_KEY", "BIGMODEL_API_KEY"];
/// 默认 OpenAI Chat 根地址；Anthropic 改成 `https://open.bigmodel.cn/api/anthropic`
pub const DEFAULT_BASE_URL: &str = "https://open.bigmodel.cn/api/paas/v4";
/// 默认模型
pub const DEFAULT_MODEL: &str = "glm-5.4";
/// 远端模型列表；没有统一 `/models` 的厂商默认不填
pub const DEFAULT_MODELS_URL: Option<&str> = None;
/// 本插件实际支持的协议
pub const SUPPORTED_PROTOCOLS: &[LlmProtocol] = &[LlmProtocol::OpenAiChat, LlmProtocol::Anthropic];
pub const DEFAULT_THINKING_MODE: ThinkingMode = ThinkingMode::Default;
pub const DEFAULT_REASONING_EFFORT: Option<ReasoningEffort> = None;
/// 是否需要在 chat 请求体里注入 `thinking.type`（目前仅 Z.ai 需要）
pub const INSERTS_THINKING_OBJECT: bool = true;

pub struct ZaiLlmPlugin {
    config: ZaiConfig,
}

impl ZaiLlmPlugin {
    pub fn new(config: ZaiConfig) -> Self {
        Self { config }
    }

    pub fn from_settings(settings: Option<&serde_json::Value>) -> CoreResult<Self> {
        Ok(Self::new(ZaiConfig::from_settings(settings)?))
    }
}

impl Default for ZaiLlmPlugin {
    fn default() -> Self {
        Self::new(ZaiConfig::default())
    }
}

#[async_trait]
impl Plugin for ZaiLlmPlugin {
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
            "智谱 Z.ai 大语言模型",
            env!("CARGO_PKG_VERSION"),
            PluginCategory::Llm,
        )
        .with_description(
            "调用智谱 Z.ai 的 Chat / Anthropic 兼容接口，注册为 LLM 驱动 zai；只接入智谱自家 GLM 模型。中国站公开文档没有 Responses 入口，因此只声明 Chat 与 Anthropic 两种协议。需要环境变量 ZAI_API_KEY。"
        )
        .with_capabilities(capabilities)
    }

    async fn apply(&self, ctx: &Context) -> CoreResult<()> {
        let Some(api_key) = self.config.api_key() else {
            tracing::warn!(
                "{DISPLAY_NAME} LLM 未注册：没有找到环境变量 {}（获取 Key：https://bigmodel.cn/usercenter/proj-mgmt/apikeys）",
                self.config.api_key_env
            );
            return Ok(());
        };
        let provider: Arc<dyn LlmProvider> =
            Arc::new(ZaiLlmProvider::new(self.config.clone(), api_key)?);
        ctx.register::<dyn LlmProvider>(DRIVER_ID, provider)?;
        tracing::info!(
            "{DISPLAY_NAME} LLM 插件加载完成，已注册驱动 {DRIVER_ID}（协议 {}，模型 {}）",
            self.config.protocol.as_str(),
            self.config.model
        );
        Ok(())
    }
}
