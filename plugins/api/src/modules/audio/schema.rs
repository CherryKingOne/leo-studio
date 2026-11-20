use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterSynthesizeRequest {
    /// 默认使用的角色声线，如 zh-CN-YunxiNeural
    #[serde(default = "default_voice")]
    pub voice: String,
    /// 语速倍率，0.5 ~ 2.0，默认 1.0
    #[serde(default = "default_speed")]
    pub speed: f32,
    /// 音调调整
    #[serde(default)]
    pub pitch: f32,
    /// 是否强制重新合成已有音频的片段
    #[serde(default)]
    pub force: bool,
}

fn default_voice() -> String {
    "zh-CN-YunxiNeural".into()
}

fn default_speed() -> f32 {
    1.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioClipItem {
    pub clip_id: String,
    pub segment_id: String,
    pub text: String,
    pub kind: String,
    pub voice: String,
    pub duration_ms: u64,
    pub audio_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterSynthesizeResponse {
    pub chapter_id: String,
    pub total_segments: usize,
    pub newly_synthesized: usize,
    pub skipped_count: usize,
    pub total_duration_ms: u64,
    pub clips: Vec<AudioClipItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceItem {
    pub id: String,
    pub name: String,
    pub gender: String,
    pub locale: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginItem {
    pub id: String,
    pub name: String,
    pub version: String,
    pub category: String,
    pub description: String,
    pub capabilities: Vec<String>,
}
