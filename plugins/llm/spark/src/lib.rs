//! 在 LLM 注册层里注册驱动 `spark`，只依赖 `leo-core` 和共享协议库
//! `leo-llm-protocol`。支持 OpenAI Chat / OpenAI Responses / Anthropic 三种协议；
//! 模型广场里还托管了 GLM / DeepSeek / Kimi / MiniMax / Qwen，本插件只放行
//! 讯飞自家的 Spark 模型。
//!
//! # 官方文档（维护时直接打开核对，不需要人工另外提供）
//! - 平台首页：<https://maas.xfyun.cn/>
//! - 快速开始 / API 接入：<https://maas.xfyun.cn/doc-viewer?url=https://maas.xfyun.cn/document/guide/1%E3%80%81%E5%BF%AB%E9%80%9F%E5%BC%80%E5%A7%8B/1.1%20API%E6%8E%A5%E5%85%A5.html>
//! - 模型广场：<https://maas.xfyun.cn/models>
//! - 获取 API Key：<https://maas.xfyun.cn/>
//!
//! # 配置
//! API Key 只从环境变量 `SPARK_API_KEY` 读取；其余参数写在
//! `"settings": {"leo-plugin-llm-spark": {...}}`，见 [`SparkConfig`]。
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

pub use client::Provider as SparkLlmProvider;
pub use config::Config as SparkConfig;
pub use models::builtin_models;

/// 插件 id
pub const PLUGIN_ID: &str = "leo-plugin-llm-spark";
/// 在 LLM 注册表里的驱动 id
pub const DRIVER_ID: &str = "spark";
/// UI / 日志里的展示名
pub const DISPLAY_NAME: &str = "讯飞星辰";

/// API Key 环境变量
pub const DEFAULT_API_KEY_ENV: &str = "SPARK_API_KEY";
/// 兼容的备用环境变量
pub const API_KEY_ENV_ALIASES: &[&str] = &["SPARK_API_PASSWORD", "XFYUN_API_KEY"];
/// 默认 OpenAI Chat 根地址；Responses 改成 `https://maas-api.cn-huabei-1.xf-yun.com/v1`，Anthropic 改成 `https://maas-api.cn-huabei-1.xf-yun.com/anthropic`
pub const DEFAULT_BASE_URL: &str = "https://maas-api.cn-huabei-1.xf-yun.com/v2";
/// 默认模型
pub const DEFAULT_MODEL: &str = "spark-x2.6";
/// 远端模型列表；没有统一 `/models` 的厂商默认不填
pub const DEFAULT_MODELS_URL: Option<&str> = None;
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

pub struct SparkLlmPlugin {
    config: SparkConfig,
}

impl SparkLlmPlugin {
    pub fn new(config: SparkConfig) -> Self {
        Self { config }
    }

    pub fn from_settings(settings: Option<&serde_json::Value>) -> CoreResult<Self> {
        Ok(Self::new(SparkConfig::from_settings(settings)?))
    }
}

impl Default for SparkLlmPlugin {
    fn default() -> Self {
        Self::new(SparkConfig::default())
    }
}

#[async_trait]
impl Plugin for SparkLlmPlugin {
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
            "讯飞星辰大语言模型",
            env!("CARGO_PKG_VERSION"),
            PluginCategory::Llm,
        )
        .with_description(
            "调用讯飞星辰 MaaS 的 Chat / Responses / Anthropic 兼容接口，注册为 LLM 驱动 spark；只接入讯飞自家 Spark 模型，模型列表会过滤 GLM / DeepSeek / Kimi 等托管模型。需要环境变量 SPARK_API_KEY。"
        )
        .with_capabilities(capabilities)
    }

    async fn apply(&self, ctx: &Context) -> CoreResult<()> {
        let Some(api_key) = self.config.api_key() else {
            tracing::warn!(
                "{DISPLAY_NAME} LLM 未注册：没有找到环境变量 {}（获取 Key：https://maas.xfyun.cn/）",
                self.config.api_key_env
            );
            return Ok(());
        };
        let provider: Arc<dyn LlmProvider> =
            Arc::new(SparkLlmProvider::new(self.config.clone(), api_key)?);
        ctx.register::<dyn LlmProvider>(DRIVER_ID, provider)?;
        tracing::info!(
            "{DISPLAY_NAME} LLM 插件加载完成，已注册驱动 {DRIVER_ID}（协议 {}，模型 {}）",
            self.config.protocol.as_str(),
            self.config.model
        );
        Ok(())
    }
}
