use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AudioFormat {
    Wav,
    Mp3,
    Opus,
    Flac,
}

impl AudioFormat {
    pub fn extension(&self) -> &'static str {
        match self {
            Self::Wav => "wav",
            Self::Mp3 => "mp3",
            Self::Opus => "opus",
            Self::Flac => "flac",
        }
    }

    pub fn mime_type(&self) -> &'static str {
        match self {
            Self::Wav => "audio/wav",
            Self::Mp3 => "audio/mpeg",
            Self::Opus => "audio/opus",
            Self::Flac => "audio/flac",
        }
    }
}

/// 音频切片元数据引用。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioClipRef {
    pub clip_id: String,
    pub format: AudioFormat,
    pub sample_rate: u32,
    pub duration_ms: u64,
    /// 音频二进制哈希或持久化相对存储路径
    pub storage_key: String,
    /// 生成该切片使用的 TTS 插件 ID 与声线 ID
    pub tts_engine: String,
    pub voice_id: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}
