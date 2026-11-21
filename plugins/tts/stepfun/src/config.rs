//! 阶跃星辰 TTS 配置。
//!
//! 参数含义与取值范围以官方文档为准：<https://platform.stepfun.com/docs/zh/api-reference/audio/create-audio>

use leo_core::error::{CoreError, CoreResult};
use serde::{Deserialize, Serialize};

/// `leo-plugins.json` → `"settings": {"leo-plugin-tts-stepfun": { ... }}`
///
/// ```json
/// {
///   "model": "step-tts-2",
///   "response_format": "mp3",
///   "sample_rate": 24000,
///   "volume": 1.0,
///   "text_normalization": "enhanced",
///   "default_voice": "cixingnansheng"
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct StepfunConfig {
    /// 读取 API Key 的环境变量名（Key 本身不写进配置文件）
    pub api_key_env: String,
    /// 接口地址；Step Plan 用户改成 `https://api.stepfun.com/step_plan/v1`
    pub base_url: String,
    /// 模型：`stepaudio-3-tts`、`stepaudio-2.5-tts`、`step-tts-2`、`step-tts-mini`
    pub model: String,
    /// 输出格式：`mp3`（默认）、`wav`、`flac`、`opus`（本插件不支持 `pcm`，因为 AST 的音频格式里没有它）
    pub response_format: String,
    /// 采样率：8000 / 16000 / 22050 / 24000 / 48000
    pub sample_rate: u32,
    /// 音量 0.1 ~ 2.0
    pub volume: f32,
    /// 文本归一化：`standard` / `enhanced`；不填不传
    pub text_normalization: Option<String>,
    /// 全局语气指导，仅 `stepaudio-2.5-tts`（≤200 字）/ `stepaudio-3-tts`（≤500 字）生效
    pub instruction: Option<String>,
    /// 请求没指定声线时用的声线
    pub default_voice: String,
    /// 单次请求超时（秒）
    pub timeout_secs: u64,
}

impl Default for StepfunConfig {
    fn default() -> Self {
        Self {
            api_key_env: "STEPFUN_API_KEY".into(),
            base_url: "https://api.stepfun.com/v1".into(),
            model: "step-tts-2".into(),
            response_format: "mp3".into(),
            sample_rate: 24000,
            volume: 1.0,
            text_normalization: None,
            instruction: None,
            default_voice: "cixingnansheng".into(),
            timeout_secs: 60,
        }
    }
}

/// 单次合成文本上限（文档：input 最大 1000 个字符）
pub const MAX_INPUT_CHARS: usize = 1000;
const SAMPLE_RATES: [u32; 5] = [8000, 16000, 22050, 24000, 48000];
const FORMATS: [&str; 4] = ["mp3", "wav", "flac", "opus"];

impl StepfunConfig {
    pub fn from_settings(settings: Option<&serde_json::Value>) -> CoreResult<Self> {
        let config: Self = match settings {
            None => Self::default(),
            Some(v) => serde_json::from_value(v.clone()).map_err(|e| {
                CoreError::InvalidParameter(format!("leo-plugin-tts-stepfun 配置格式错误：{e}"))
            })?,
        };
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> CoreResult<()> {
        let bad = |m: String| Err(CoreError::InvalidParameter(format!("leo-plugin-tts-stepfun 配置错误：{m}")));
        if !FORMATS.contains(&self.response_format.as_str()) {
            return bad(format!("response_format 只支持 {FORMATS:?}，收到 {}", self.response_format));
        }
        if !SAMPLE_RATES.contains(&self.sample_rate) {
            return bad(format!("sample_rate 只支持 {SAMPLE_RATES:?}，收到 {}", self.sample_rate));
        }
        if !(0.1..=2.0).contains(&self.volume) {
            return bad(format!("volume 范围 0.1 ~ 2.0，收到 {}", self.volume));
        }
        if let Some(n) = &self.text_normalization {
            if n != "standard" && n != "enhanced" {
                return bad(format!("text_normalization 只支持 standard / enhanced，收到 {n}"));
            }
        }
        Ok(())
    }

    /// 从环境变量读 API Key
    pub fn api_key(&self) -> Option<String> {
        std::env::var(&self.api_key_env).ok().map(|k| k.trim().to_string()).filter(|k| !k.is_empty())
    }

    /// 是否为支持 instruction / 不支持 voice_label 的 StepAudio 系列模型
    pub fn is_stepaudio(&self) -> bool {
        self.model.starts_with("stepaudio-")
    }
}
