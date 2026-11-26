//! leo-plugin-llm-bailian：阿里云百炼（Qwen）大语言模型子插件（“小盒子”）。
//!
//! 在 LLM 注册层（leo-plugin-llm）里注册驱动 id [`DRIVER_ID`]（`bailian`），只依赖
//! `leo-core` 和共享协议库 `leo-llm-protocol`。
//!
//! 用法：模型写 `bailian:qwen-plus`；或在 `leo-plugins.json` 把
//! `"llm": {"default": "bailian"}` 设成默认驱动。
//!
//! # 官方文档（维护时直接打开这些链接核对接口，不需要人工另外提供）
//! - 百炼文档入口：<https://help.aliyun.com/zh/model-studio/>
//! - OpenAI 兼容接口：<https://help.aliyun.com/zh/model-studio/compatibility-of-openai-with-dashscope>
//! - 模型列表：<https://help.aliyun.com/zh/model-studio/models>
//! - 获取 API Key：<https://bailian.console.aliyun.com/>
//!
//! # 配置
//! API Key 只从环境变量读取（默认 `DASHSCOPE_API_KEY`），不写进配置文件。其余参数写在
//! `leo-plugins.json` 的 `"settings": {"leo-plugin-llm-bailian": {...}}` 里，见 [`BailianConfig`]。
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

pub use client::BailianLlmProvider;
pub use config::BailianConfig;
pub use models::builtin_models;

/// 插件 id
pub const PLUGIN_ID: &str = "leo-plugin-llm-bailian";
/// 在 LLM 注册表里的驱动 id
pub const DRIVER_ID: &str = "bailian";

pub struct BailianLlmPlugin {
    config: BailianConfig,
}

impl BailianLlmPlugin {
    pub fn new(config: BailianConfig) -> Self {
        Self { config }
    }

    /// 从 `leo-plugins.json` 的 settings 一节解析配置（没有这一节时用默认配置）
    pub fn from_settings(settings: Option<&serde_json::Value>) -> CoreResult<Self> {
        Ok(Self::new(BailianConfig::from_settings(settings)?))
    }
}

impl Default for BailianLlmPlugin {
    fn default() -> Self {
        Self::new(BailianConfig::default())
    }
}

#[async_trait]
impl Plugin for BailianLlmPlugin {
    fn manifest(&self) -> PluginManifest {
        PluginManifest::new(
            PLUGIN_ID,
            "阿里云百炼（Qwen）大语言模型",
            env!("CARGO_PKG_VERSION"),
            PluginCategory::Llm,
        )
        .with_description(
            "调用阿里云百炼 OpenAI 兼容 / Responses / Anthropic 接口，注册为 LLM 驱动 bailian。需要环境变量 DASHSCOPE_API_KEY。",
        )
        .with_capabilities(["llm:chat", "llm:list_models", "llm:protocol:openai-chat", "llm:protocol:openai-responses", "llm:protocol:anthropic"])
    }

    async fn apply(&self, ctx: &Context) -> CoreResult<()> {
        let Some(api_key) = self.config.api_key() else {
            tracing::warn!(
                "阿里云百炼 LLM 未注册：没有找到环境变量 {}（获取 Key：https://bailian.console.aliyun.com/）",
                self.config.api_key_env
            );
            return Ok(());
        };
        let provider: Arc<dyn LlmProvider> =
            Arc::new(BailianLlmProvider::new(self.config.clone(), api_key)?);
        ctx.register::<dyn LlmProvider>(DRIVER_ID, provider)?;
        tracing::info!(
            "阿里云百炼 LLM 插件加载完成，已注册 LLM 驱动 {DRIVER_ID}（协议 {}，模型 {}）",
            self.config.protocol.as_str(),
            self.config.model
        );
        Ok(())
    }
}
