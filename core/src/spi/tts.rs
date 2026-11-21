use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::ast::audio::AudioFormat;
use crate::error::CoreResult;

/// 语音合成输入载荷
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynthesisPayload {
    pub text: String,
    /// 目标声线（交给具体驱动时已去掉 `驱动id:` 前缀，如 zh-CN-XiaoxiaoNeural）
    pub voice: String,
    /// 语速倍率 0.5 ~ 2.0
    pub speed: f32,
    /// 音调调整 -100% ~ +100%
    pub pitch: f32,
    /// 情绪标识
    pub emotion: Option<String>,
}

/// 语音合成结果
#[derive(Debug, Clone)]
pub struct SynthesisOutput {
    pub audio_bytes: Vec<u8>,
    pub format: AudioFormat,
    pub sample_rate: u32,
    pub duration_ms: u64,
}

/// 声线信息描述
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceInfo {
    pub id: String,
    pub name: String,
    pub gender: String,
    pub locale: String,
    pub description: String,
}

/// TTS 语音合成扩展点
#[async_trait]
pub trait TTSProvider: Send + Sync {
    fn provider_id(&self) -> &str;
    async fn list_voices(&self) -> CoreResult<Vec<VoiceInfo>>;
    async fn synthesize(&self, payload: &SynthesisPayload) -> CoreResult<SynthesisOutput>;
}
