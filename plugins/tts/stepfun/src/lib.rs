//! leo-plugin-tts-stepfun：阶跃星辰（StepFun）语音合成子插件（“小盒子”）。
//!
//! 在 TTS 注册层（leo-plugin-tts）里注册驱动 id [`DRIVER_ID`]（`stepfun`），只依赖 `leo-core`。
//! 用法：声线写 `stepfun:cixingnansheng`；或在 `leo-plugins.json` 把 `"tts": {"default": "stepfun"}` 设成默认驱动。
//!
//! # 官方文档（维护时直接打开这些链接核对接口，不需要人工另外提供）
//! - 文档总索引（列出全部页面，先看这里找新页面）：<https://platform.stepfun.com/docs/llms.txt>
//! - 语音合成接口 `POST /v1/audio/speech`：<https://platform.stepfun.com/docs/zh/api-reference/audio/create-audio>
//!   （Markdown 版：<https://platform.stepfun.com/docs/zh/api-reference/audio/create-audio.md>）
//! - 官方音色列表接口 `GET /v1/audio/system_voices`：<https://platform.stepfun.com/docs/zh/api-reference/audio/system-voices>
//! - 音频合成开发指南（官方音色清单、音色标签）：<https://platform.stepfun.com/docs/zh/guides/developer/tts>
//! - 流式语音合成（WebSocket，本插件暂未使用）：<https://platform.stepfun.com/docs/zh/api-reference/audio/ws-audio>
//! - 错误码：<https://platform.stepfun.com/docs/zh/api-reference/error-codes>
//! - API 参考入口（用户给的链接）：<https://platform.stepfun.com/docs/zh/api-reference/chat/chat-completion-create>
//! - 获取 API Key：<https://platform.stepfun.com/interface-key>
//!
//! # 配置
//! API Key 只从环境变量读取（默认 `STEPFUN_API_KEY`），不写进配置文件。其余参数写在
//! `leo-plugins.json` 的 `"settings": {"leo-plugin-tts-stepfun": {...}}` 里，见 [`StepfunConfig`]。
//! 没有 API Key 时插件照常装配但不注册驱动（只打一条警告），不影响其他插件。

mod audio;
mod client;
mod config;
mod voices;

use std::sync::Arc;

use async_trait::async_trait;
use leo_core::context::Context;
use leo_core::error::CoreResult;
use leo_core::plugin::{Plugin, PluginCategory, PluginManifest};
use leo_core::spi::TTSProvider;

pub use audio::probe_duration_ms;
pub use client::StepfunTtsProvider;
pub use config::StepfunConfig;
pub use voices::builtin_voices;

/// 插件 id
pub const PLUGIN_ID: &str = "leo-plugin-tts-stepfun";
/// 在 TTS 注册表里的驱动 id
pub const DRIVER_ID: &str = "stepfun";

pub struct StepfunTtsPlugin {
    config: StepfunConfig,
}

impl StepfunTtsPlugin {
    pub fn new(config: StepfunConfig) -> Self {
        Self { config }
    }

    /// 从 `leo-plugins.json` 的 settings 一节解析配置（没有这一节时用默认配置）
    pub fn from_settings(settings: Option<&serde_json::Value>) -> CoreResult<Self> {
        Ok(Self::new(StepfunConfig::from_settings(settings)?))
    }
}

impl Default for StepfunTtsPlugin {
    fn default() -> Self {
        Self::new(StepfunConfig::default())
    }
}

#[async_trait]
impl Plugin for StepfunTtsPlugin {
    fn manifest(&self) -> PluginManifest {
        PluginManifest::new(PLUGIN_ID, "阶跃星辰 StepFun 语音合成", env!("CARGO_PKG_VERSION"), PluginCategory::Tts)
            .with_description("调用阶跃星辰 /v1/audio/speech 合成语音，注册为 TTS 驱动 stepfun。需要环境变量 STEPFUN_API_KEY。")
            .with_capabilities(["tts:synthesize", "tts:list_voices"])
    }

    async fn apply(&self, ctx: &Context) -> CoreResult<()> {
        let Some(api_key) = self.config.api_key() else {
            tracing::warn!(
                "阶跃星辰 TTS 未注册：没有找到环境变量 {}（获取 Key：https://platform.stepfun.com/interface-key）",
                self.config.api_key_env
            );
            return Ok(());
        };
        let provider: Arc<dyn TTSProvider> = Arc::new(StepfunTtsProvider::new(self.config.clone(), api_key)?);
        ctx.register::<dyn TTSProvider>(DRIVER_ID, provider)?;
        tracing::info!("阶跃星辰 TTS 插件加载完成，已注册 TTS 驱动 {DRIVER_ID}（模型 {}）", self.config.model);
        Ok(())
    }
}
