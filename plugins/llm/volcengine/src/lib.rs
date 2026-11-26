//! leo-plugin-llm-volcengine：火山引擎方舟（豆包）大语言模型子插件（“小盒子”）。
//!
//! 在 LLM 注册层（leo-plugin-llm）里注册驱动 id [`DRIVER_ID`]（`volcengine`），只依赖
//! `leo-core` 和共享协议库 `leo-llm-protocol`。
//!
//! 用法：模型写 `volcengine:<推理接入点ID或模型ID>`；或在 `leo-plugins.json` 把
//! `"llm": {"default": "volcengine"}` 设成默认驱动。
//!
//! # 官方文档（维护时直接打开这些链接核对接口，不需要人工另外提供）
//! - 火山方舟文档入口：<https://www.volcengine.com/docs/82379>
//! - Chat Completions：<https://www.volcengine.com/docs/82379/1298454>
//! - 推理接入点（Endpoint ID）：<https://www.volcengine.com/docs/82379/1099522>
//! - 获取 API Key：<https://console.volcengine.com/ark>
//!
//! # 配置
//! API Key 只从环境变量读取（默认 `ARK_API_KEY`），不写进配置文件。其余参数写在
//! `leo-plugins.json` 的 `"settings": {"leo-plugin-llm-volcengine": {...}}` 里，见 [`VolcengineConfig`]。
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

pub use client::VolcengineLlmProvider;
pub use config::VolcengineConfig;
pub use models::builtin_models;

/// 插件 id
pub const PLUGIN_ID: &str = "leo-plugin-llm-volcengine";
/// 在 LLM 注册表里的驱动 id
pub const DRIVER_ID: &str = "volcengine";

pub struct VolcengineLlmPlugin {
    config: VolcengineConfig,
}

impl VolcengineLlmPlugin {
    pub fn new(config: VolcengineConfig) -> Self {
        Self { config }
    }

    /// 从 `leo-plugins.json` 的 settings 一节解析配置（没有这一节时用默认配置）
    pub fn from_settings(settings: Option<&serde_json::Value>) -> CoreResult<Self> {
        Ok(Self::new(VolcengineConfig::from_settings(settings)?))
    }
}

impl Default for VolcengineLlmPlugin {
    fn default() -> Self {
        Self::new(VolcengineConfig::default())
    }
}

#[async_trait]
impl Plugin for VolcengineLlmPlugin {
    fn manifest(&self) -> PluginManifest {
        PluginManifest::new(
            PLUGIN_ID,
            "火山方舟（豆包）大语言模型",
            env!("CARGO_PKG_VERSION"),
            PluginCategory::Llm,
        )
        .with_description(
            "调用火山引擎方舟的 Chat Completions / Responses / Anthropic 兼容接口，注册为 LLM 驱动 volcengine。需要环境变量 ARK_API_KEY。",
        )
        .with_capabilities(["llm:chat", "llm:list_models", "llm:protocol:openai-chat", "llm:protocol:openai-responses", "llm:protocol:anthropic"])
    }

    async fn apply(&self, ctx: &Context) -> CoreResult<()> {
        let Some(api_key) = self.config.api_key() else {
            tracing::warn!(
                "火山方舟 LLM 未注册：没有找到环境变量 {}（获取 Key：https://console.volcengine.com/ark）",
                self.config.api_key_env
            );
            return Ok(());
        };
        let provider: Arc<dyn LlmProvider> =
            Arc::new(VolcengineLlmProvider::new(self.config.clone(), api_key)?);
        ctx.register::<dyn LlmProvider>(DRIVER_ID, provider)?;
        tracing::info!(
            "火山方舟 LLM 插件加载完成，已注册 LLM 驱动 {DRIVER_ID}（协议 {}，模型 {}）",
            self.config.protocol.as_str(),
            self.config.model
        );
        Ok(())
    }
}
