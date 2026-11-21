//! MiniMax TTS 配置。
//!
//! 参数含义与取值范围以官方文档为准：<https://platform.minimax.cn/docs/api-reference/speech-t2a-http>

use leo_core::error::{CoreError, CoreResult};
use serde::{Deserialize, Serialize};

/// `leo-plugins.json` → `"settings": {"leo-plugin-tts-minimax": { ... }}`
///
/// ```json
/// {
///   "model": "speech-2.8-hd",
///   "format": "mp3",
///   "sample_rate": 32000,
///   "bitrate": 128000,
///   "language_boost": "Chinese",
///   "default_voice": "male-qn-qingse"
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct MinimaxConfig {
    /// 读取 API Key 的环境变量名（Key 本身不写进配置文件）
    pub api_key_env: String,
    /// 接口地址（不带 /v1）。华北等其他接入地址见同步语音合成指南的“接入地址”一节
    pub base_url: String,
    /// 模型：speech-2.8-hd / speech-2.8-turbo / speech-2.6-hd / speech-2.6-turbo / speech-02-hd / speech-02-turbo 等
    pub model: String,
    /// 输出格式：mp3（默认）/ wav / flac / opus（本插件不支持 pcm 系列，因为 AST 的音频格式里没有它）
    pub format: String,
    /// 采样率：8000 / 16000 / 22050 / 24000 / 32000 / 44100
    pub sample_rate: u32,
    /// 比特率（仅 mp3）：32000 / 64000 / 128000 / 256000
    pub bitrate: u32,
    /// 声道：1 / 2
    pub channel: u8,
    /// 音量 (0, 10]
    pub vol: f32,
    /// 小语种 / 方言增强，如 `Chinese`、`Chinese,Yue`、`auto`；不填不传
    pub language_boost: Option<String>,
    /// 文本归一化：`basic` / `quality`；不填不传
    pub text_normalization_mode: Option<String>,
    /// 请求没指定声线时用的声线
    pub default_voice: String,
    /// 单次请求超时（秒）
    pub timeout_secs: u64,
}

impl Default for MinimaxConfig {
    fn default() -> Self {
        Self {
            api_key_env: "MINIMAX_API_KEY".into(),
            base_url: "https://api.minimax.cn".into(),
            model: "speech-2.8-hd".into(),
            format: "mp3".into(),
            sample_rate: 32000,
            bitrate: 128000,
            channel: 1,
            vol: 1.0,
            language_boost: None,
            text_normalization_mode: None,
            default_voice: "male-qn-qingse".into(),
            timeout_secs: 120,
        }
    }
}

/// 单次合成文本上限（文档：长度限制小于 10000 字符）
pub const MAX_TEXT_CHARS: usize = 9999;
const SAMPLE_RATES: [u32; 6] = [8000, 16000, 22050, 24000, 32000, 44100];
const BITRATES: [u32; 4] = [32000, 64000, 128000, 256000];
const FORMATS: [&str; 4] = ["mp3", "wav", "flac", "opus"];

impl MinimaxConfig {
    pub fn from_settings(settings: Option<&serde_json::Value>) -> CoreResult<Self> {
        let config: Self = match settings {
            None => Self::default(),
            Some(v) => serde_json::from_value(v.clone()).map_err(|e| {
                CoreError::InvalidParameter(format!("leo-plugin-tts-minimax 配置格式错误：{e}"))
            })?,
        };
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> CoreResult<()> {
        let bad = |m: String| Err(CoreError::InvalidParameter(format!("leo-plugin-tts-minimax 配置错误：{m}")));
        if !FORMATS.contains(&self.format.as_str()) {
            return bad(format!("format 只支持 {FORMATS:?}，收到 {}", self.format));
        }
        if !SAMPLE_RATES.contains(&self.sample_rate) {
            return bad(format!("sample_rate 只支持 {SAMPLE_RATES:?}，收到 {}", self.sample_rate));
        }
        if !BITRATES.contains(&self.bitrate) {
            return bad(format!("bitrate 只支持 {BITRATES:?}，收到 {}", self.bitrate));
        }
        if self.channel != 1 && self.channel != 2 {
            return bad(format!("channel 只支持 1 / 2，收到 {}", self.channel));
        }
        if !(self.vol > 0.0 && self.vol <= 10.0) {
            return bad(format!("vol 范围 (0, 10]，收到 {}", self.vol));
        }
        if let Some(m) = &self.text_normalization_mode {
            if m != "basic" && m != "quality" {
                return bad(format!("text_normalization_mode 只支持 basic / quality，收到 {m}"));
            }
        }
        Ok(())
    }

    /// 从环境变量读 API Key
    pub fn api_key(&self) -> Option<String> {
        std::env::var(&self.api_key_env).ok().map(|k| k.trim().to_string()).filter(|k| !k.is_empty())
    }
}
