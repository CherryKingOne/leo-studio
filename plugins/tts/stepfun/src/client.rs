//! 阶跃星辰 HTTP 接口调用。
//!
//! - 合成：`POST {base_url}/audio/speech`，返回音频二进制
//!   <https://platform.stepfun.com/docs/zh/api-reference/audio/create-audio>
//! - 音色：`GET {base_url}/audio/system_voices?model=step-tts-2`
//!   <https://platform.stepfun.com/docs/zh/api-reference/audio/system-voices>
//! - 错误码：<https://platform.stepfun.com/docs/zh/api-reference/error-codes>

use std::collections::HashMap;
use std::time::Duration;

use async_trait::async_trait;
use leo_core::ast::AudioFormat;
use leo_core::error::{CoreError, CoreResult};
use leo_core::spi::{SynthesisOutput, SynthesisPayload, TTSProvider, VoiceInfo};
use serde::Deserialize;
use serde_json::{Map, Value, json};

use crate::audio::probe_duration_ms;
use crate::config::{MAX_INPUT_CHARS, StepfunConfig};
use crate::voices::{builtin_voices, gender_from_description};
use crate::PLUGIN_ID;

/// 系统音色接口目前只支持这个模型（见 system-voices 文档）
const SYSTEM_VOICES_MODEL: &str = "step-tts-2";

pub struct StepfunTtsProvider {
    config: StepfunConfig,
    api_key: String,
    http: reqwest::Client,
}

fn plugin_err(message: impl Into<String>) -> CoreError {
    CoreError::Plugin { plugin_id: PLUGIN_ID.into(), message: message.into() }
}

impl StepfunTtsProvider {
    pub fn new(config: StepfunConfig, api_key: String) -> CoreResult<Self> {
        config.validate()?;
        // reqwest 用的是 no-provider 版 rustls：这里装上进程级默认加密后端（aws-lc-rs，
        // 与 msedge-tts 一致）。已经装过时 install_default 返回 Err，忽略即可。
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs.max(1)))
            .build()
            .map_err(|e| plugin_err(format!("创建 HTTP 客户端失败：{e}")))?;
        Ok(Self { config, api_key, http })
    }

    fn url(&self, path: &str) -> String {
        format!("{}/{}", self.config.base_url.trim_end_matches('/'), path.trim_start_matches('/'))
    }

    /// 组装 `/audio/speech` 请求体（单独拿出来方便测试）
    pub fn build_request(&self, payload: &SynthesisPayload) -> CoreResult<Value> {
        let text = payload.text.trim();
        if text.is_empty() {
            return Err(CoreError::InvalidParameter("合成文本为空".into()));
        }
        let chars = text.chars().count();
        if chars > MAX_INPUT_CHARS {
            return Err(CoreError::InvalidParameter(format!(
                "阶跃星辰单次最多合成 {MAX_INPUT_CHARS} 个字符，这段有 {chars} 个，请先切句"
            )));
        }
        let voice = if payload.voice.trim().is_empty() { self.config.default_voice.as_str() } else { payload.voice.trim() };

        let mut body = Map::new();
        body.insert("model".into(), json!(self.config.model));
        body.insert("input".into(), json!(text));
        body.insert("voice".into(), json!(voice));
        body.insert("response_format".into(), json!(self.config.response_format));
        body.insert("sample_rate".into(), json!(self.config.sample_rate));
        // speed：文档范围 0.5 ~ 2，默认 1.0
        body.insert("speed".into(), json!(payload.speed.clamp(0.5, 2.0)));
        body.insert("volume".into(), json!(self.config.volume));
        if let Some(n) = &self.config.text_normalization {
            body.insert("text_normalization".into(), json!(n));
        }
        // 情绪：StepAudio 系列用 instruction（不支持 voice_label）；其他模型用 voice_label.emotion
        // （voice_label 文档说明是“使用自定义音色时需要传入”，官方音色传了不生效也不报错）
        let emotion = payload.emotion.as_deref().map(str::trim).filter(|e| !e.is_empty());
        if self.config.is_stepaudio() {
            let instruction = match (&self.config.instruction, emotion) {
                (Some(base), Some(e)) => Some(format!("{base}；情绪：{e}")),
                (Some(base), None) => Some(base.clone()),
                (None, Some(e)) => Some(format!("情绪：{e}")),
                (None, None) => None,
            };
            if let Some(i) = instruction {
                body.insert("instruction".into(), json!(i));
            }
        } else if let Some(e) = emotion {
            body.insert("voice_label".into(), json!({ "emotion": e }));
        }
        // pitch：接口没有音调参数，忽略
        Ok(Value::Object(body))
    }

    fn audio_format(&self) -> AudioFormat {
        match self.config.response_format.as_str() {
            "wav" => AudioFormat::Wav,
            "flac" => AudioFormat::Flac,
            "opus" => AudioFormat::Opus,
            _ => AudioFormat::Mp3,
        }
    }
}

/// 错误响应体（OpenAI 风格），解析失败时原样返回文本
fn error_message(status: reqwest::StatusCode, body: &str) -> String {
    #[derive(Deserialize)]
    struct ErrBody {
        error: Option<ErrInner>,
        message: Option<String>,
    }
    #[derive(Deserialize)]
    struct ErrInner {
        message: Option<String>,
        #[serde(rename = "type")]
        kind: Option<String>,
    }
    let detail = serde_json::from_str::<ErrBody>(body)
        .ok()
        .and_then(|b| match b.error {
            Some(e) => Some(format!("{}{}", e.kind.map(|k| format!("[{k}] ")).unwrap_or_default(), e.message.unwrap_or_default())),
            None => b.message,
        })
        .unwrap_or_else(|| body.chars().take(300).collect());
    format!("HTTP {status}：{detail}")
}

#[derive(Deserialize)]
struct SystemVoices {
    #[serde(default)]
    voices: Vec<String>,
    #[serde(rename = "voices-details", default)]
    details: HashMap<String, VoiceDetail>,
}

#[derive(Deserialize, Default)]
struct VoiceDetail {
    #[serde(rename = "voice-name", default)]
    name: String,
    #[serde(rename = "voice-description", default)]
    description: String,
    #[serde(default)]
    recommended_scene: String,
}

#[async_trait]
impl TTSProvider for StepfunTtsProvider {
    fn provider_id(&self) -> &str {
        crate::DRIVER_ID
    }

    async fn list_voices(&self) -> CoreResult<Vec<VoiceInfo>> {
        let resp = self
            .http
            .get(self.url("audio/system_voices"))
            .query(&[("model", SYSTEM_VOICES_MODEL)])
            .bearer_auth(&self.api_key)
            .send()
            .await;
        let parsed = match resp {
            Ok(r) if r.status().is_success() => r.json::<SystemVoices>().await.map_err(|e| e.to_string()),
            Ok(r) => {
                let status = r.status();
                Err(error_message(status, &r.text().await.unwrap_or_default()))
            }
            Err(e) => Err(e.to_string()),
        };
        match parsed {
            Ok(list) if !list.voices.is_empty() => Ok(list
                .voices
                .into_iter()
                .map(|id| {
                    let d = list.details.get(&id);
                    let name = d.map(|d| d.name.clone()).filter(|n| !n.is_empty()).unwrap_or_else(|| id.clone());
                    let desc = d.map(|d| d.description.as_str()).unwrap_or("");
                    let scene = d.map(|d| d.recommended_scene.as_str()).unwrap_or("");
                    VoiceInfo {
                        gender: gender_from_description(desc).into(),
                        locale: "zh-CN".into(),
                        description: [desc, scene].iter().filter(|s| !s.is_empty()).cloned().collect::<Vec<_>>().join("；"),
                        name,
                        id,
                    }
                })
                .collect()),
            Ok(_) => Ok(builtin_voices()),
            Err(e) => {
                tracing::warn!("获取阶跃星辰官方音色失败，改用内置列表：{e}");
                Ok(builtin_voices())
            }
        }
    }

    async fn synthesize(&self, payload: &SynthesisPayload) -> CoreResult<SynthesisOutput> {
        let body = self.build_request(payload)?;
        let resp = self
            .http
            .post(self.url("audio/speech"))
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| plugin_err(format!("请求阶跃星辰语音合成失败：{e}")))?;
        let status = resp.status();
        let is_json = resp
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .is_some_and(|v| v.contains("json"));
        let bytes = resp.bytes().await.map_err(|e| plugin_err(format!("读取阶跃星辰响应失败：{e}")))?;
        if !status.is_success() || is_json {
            return Err(plugin_err(format!("阶跃星辰语音合成失败，{}", error_message(status, &String::from_utf8_lossy(&bytes)))));
        }
        if bytes.is_empty() {
            return Err(plugin_err("阶跃星辰返回了空音频"));
        }
        let audio_bytes = bytes.to_vec();
        let duration_ms = probe_duration_ms(&audio_bytes).unwrap_or(0);
        Ok(SynthesisOutput { format: self.audio_format(), sample_rate: self.config.sample_rate, duration_ms, audio_bytes })
    }
}
