//! 在 LLM 注册层里注册驱动 `qianfan`，只依赖 `leo-core` 和共享协议库
//! `leo-llm-protocol`。千帆公开文档目前只提供 OpenAI Chat 兼容入口，
//! 因此本插件只声明 OpenAI Chat 一种协议；只放行百度自家 `ernie-*` 模型。
//!
//! # 官方文档（维护时直接打开核对，不需要人工另外提供）
//! - API 文档：<https://cloud.baidu.com/doc/qianfan-api/s/3m7of64lb>
//! - 获取 API Key：<https://console.bce.baidu.com/iam/#/iam/apikey/list>
//!
//! # 配置
//! API Key 只从环境变量 `QIANFAN_API_KEY` 读取；其余参数写在
//! `"settings": {"leo-plugin-llm-qianfan": {...}}`，见 [`QianfanConfig`]。
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

pub use client::Provider as QianfanLlmProvider;
pub use config::Config as QianfanConfig;
pub use models::builtin_models;

/// 插件 id
pub const PLUGIN_ID: &str = "leo-plugin-llm-qianfan";
/// 在 LLM 注册表里的驱动 id
pub const DRIVER_ID: &str = "qianfan";
/// UI / 日志里的展示名
pub const DISPLAY_NAME: &str = "百度千帆";

/// API Key 环境变量
pub const DEFAULT_API_KEY_ENV: &str = "QIANFAN_API_KEY";
/// 兼容的备用环境变量
pub const API_KEY_ENV_ALIASES: &[&str] = &["BAIDU_QIANFAN_API_KEY"];
/// OpenAI Chat 根地址；千帆公开文档没有 Responses / Anthropic 入口
pub const DEFAULT_BASE_URL: &str = "https://qianfan.baidubce.com/v2";
/// 默认模型
pub const DEFAULT_MODEL: &str = "ernie-5.5-flash-preview";
/// 远端模型列表；没有统一 `/models` 的厂商默认不填
pub const DEFAULT_MODELS_URL: Option<&str> = None;
/// 本插件实际支持的协议
pub const SUPPORTED_PROTOCOLS: &[LlmProtocol] = &[LlmProtocol::OpenAiChat];
pub const DEFAULT_THINKING_MODE: ThinkingMode = ThinkingMode::Default;
pub const DEFAULT_REASONING_EFFORT: Option<ReasoningEffort> = None;
/// 是否需要在 chat 请求体里注入 `thinking.type`（目前仅 Z.ai 需要）
pub const INSERTS_THINKING_OBJECT: bool = false;

pub struct QianfanLlmPlugin {
    config: QianfanConfig,
}

impl QianfanLlmPlugin {
    pub fn new(config: QianfanConfig) -> Self {
        Self { config }
    }

    pub fn from_settings(settings: Option<&serde_json::Value>) -> CoreResult<Self> {
        Ok(Self::new(QianfanConfig::from_settings(settings)?))
    }
}

impl Default for QianfanLlmPlugin {
    fn default() -> Self {
        Self::new(QianfanConfig::default())
    }
}

#[async_trait]
impl Plugin for QianfanLlmPlugin {
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
            "百度千帆大语言模型",
            env!("CARGO_PKG_VERSION"),
            PluginCategory::Llm,
        )
        .with_description(
            "调用百度千帆的 OpenAI Chat 兼容接口，注册为 LLM 驱动 qianfan；只接入百度自家 ERNIE 模型，模型列表会过滤千帆托管的 DeepSeek / Qwen 等第三方模型。需要环境变量 QIANFAN_API_KEY。"
        )
        .with_capabilities(capabilities)
    }

    async fn apply(&self, ctx: &Context) -> CoreResult<()> {
        let Some(api_key) = self.config.api_key() else {
            tracing::warn!(
                "{DISPLAY_NAME} LLM 未注册：没有找到环境变量 {}（获取 Key：https://console.bce.baidu.com/iam/#/iam/apikey/list）",
                self.config.api_key_env
            );
            return Ok(());
        };
        let provider: Arc<dyn LlmProvider> =
            Arc::new(QianfanLlmProvider::new(self.config.clone(), api_key)?);
        ctx.register::<dyn LlmProvider>(DRIVER_ID, provider)?;
        tracing::info!(
            "{DISPLAY_NAME} LLM 插件加载完成，已注册驱动 {DRIVER_ID}（协议 {}，模型 {}）",
            self.config.protocol.as_str(),
            self.config.model
        );
        Ok(())
    }
}
