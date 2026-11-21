//! leo-plugin-tts-minimax：MiniMax 语音合成子插件（“小盒子”）。
//!
//! 在 TTS 注册层（leo-plugin-tts）里注册驱动 id [`DRIVER_ID`]（`minimax`），只依赖 `leo-core`。
//! 用法：声线写 `minimax:male-qn-qingse`；或在 `leo-plugins.json` 把 `"tts": {"default": "minimax"}` 设成默认驱动。
//!
//! # 官方文档（维护时直接打开这些链接核对接口，不需要人工另外提供）
//! - 文档总索引（列出全部页面，先看这里找新页面）：<https://platform.minimax.cn/docs/llms.txt>
//! - 接口概览（用户给的链接，含语音模型列表与能力说明）：<https://platform.minimax.cn/docs/api-reference/api-overview>
//! - 同步语音合成 HTTP `POST /v1/t2a_v2`：<https://platform.minimax.cn/docs/api-reference/speech-t2a-http>
//!   （Markdown + OpenAPI 版：<https://platform.minimax.cn/docs/api-reference/speech-t2a-http.md>）
//! - 同步语音合成指南（接入地址、文本归一化、停顿标记）：<https://platform.minimax.cn/docs/guides/speech-t2a-websocket>
//! - 查询可用音色 `POST /v1/get_voice`：<https://platform.minimax.cn/docs/api-reference/voice-management-get>
//! - 系统音色列表：<https://platform.minimax.cn/docs/faq/system-voice-id>
//! - 异步长文本语音合成（整本书，本插件暂未使用）：<https://platform.minimax.cn/docs/api-reference/speech-t2a-async-create>
//! - 错误码：<https://platform.minimax.cn/docs/api-reference/errorcode>
//! - 速率限制：<https://platform.minimax.cn/docs/guides/rate-limits>
//! - 获取 API Key：<https://platform.minimax.cn/user-center/basic-information/interface-key>
//!
//! # 配置
//! API Key 只从环境变量读取（默认 `MINIMAX_API_KEY`），不写进配置文件。其余参数写在
//! `leo-plugins.json` 的 `"settings": {"leo-plugin-tts-minimax": {...}}` 里，见 [`MinimaxConfig`]。
//! 没有 API Key 时插件照常装配但不注册驱动（只打一条警告），不影响其他插件。

mod client;
mod config;
mod voices;

use std::sync::Arc;

use async_trait::async_trait;
use leo_core::context::Context;
use leo_core::error::CoreResult;
use leo_core::plugin::{Plugin, PluginCategory, PluginManifest};
use leo_core::spi::TTSProvider;

pub use client::{MinimaxTtsProvider, decode_hex};
pub use config::MinimaxConfig;
pub use voices::{builtin_voices, map_emotion, pitch_to_semitones};

/// 插件 id
pub const PLUGIN_ID: &str = "leo-plugin-tts-minimax";
/// 在 TTS 注册表里的驱动 id
pub const DRIVER_ID: &str = "minimax";

pub struct MinimaxTtsPlugin {
    config: MinimaxConfig,
}

impl MinimaxTtsPlugin {
    pub fn new(config: MinimaxConfig) -> Self {
        Self { config }
    }

    /// 从 `leo-plugins.json` 的 settings 一节解析配置（没有这一节时用默认配置）
    pub fn from_settings(settings: Option<&serde_json::Value>) -> CoreResult<Self> {
        Ok(Self::new(MinimaxConfig::from_settings(settings)?))
    }
}

impl Default for MinimaxTtsPlugin {
    fn default() -> Self {
        Self::new(MinimaxConfig::default())
    }
}

#[async_trait]
impl Plugin for MinimaxTtsPlugin {
    fn manifest(&self) -> PluginManifest {
        PluginManifest::new(PLUGIN_ID, "MiniMax 语音合成", env!("CARGO_PKG_VERSION"), PluginCategory::Tts)
            .with_description("调用 MiniMax /v1/t2a_v2 同步语音合成，注册为 TTS 驱动 minimax。需要环境变量 MINIMAX_API_KEY。")
            .with_capabilities(["tts:synthesize", "tts:list_voices"])
    }

    async fn apply(&self, ctx: &Context) -> CoreResult<()> {
        let Some(api_key) = self.config.api_key() else {
            tracing::warn!(
                "MiniMax TTS 未注册：没有找到环境变量 {}（获取 Key：https://platform.minimax.cn/user-center/basic-information/interface-key）",
                self.config.api_key_env
            );
            return Ok(());
        };
        let provider: Arc<dyn TTSProvider> = Arc::new(MinimaxTtsProvider::new(self.config.clone(), api_key)?);
        ctx.register::<dyn TTSProvider>(DRIVER_ID, provider)?;
        tracing::info!("MiniMax TTS 插件加载完成，已注册 TTS 驱动 {DRIVER_ID}（模型 {}）", self.config.model);
        Ok(())
    }
}
