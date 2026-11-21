//! MiniMax HTTP 接口调用。
//!
//! - 合成：`POST {base_url}/v1/t2a_v2`（非流式，`output_format=hex`，音频在 `data.audio`，十六进制）
//!   <https://platform.minimax.cn/docs/api-reference/speech-t2a-http>
//! - 音色：`POST {base_url}/v1/get_voice`，`{"voice_type": "system"}`
//!   <https://platform.minimax.cn/docs/api-reference/voice-management-get>
//! - 业务错误放在 `base_resp.status_code`（非 0 即失败），HTTP 状态码可能仍是 200；错误码含义：
//!   <https://platform.minimax.cn/docs/api-reference/errorcode>

use std::time::Duration;

use async_trait::async_trait;
use leo_core::ast::AudioFormat;
use leo_core::error::{CoreError, CoreResult};
use leo_core::spi::{SynthesisOutput, SynthesisPayload, TTSProvider, VoiceInfo};
use serde::Deserialize;
use serde_json::{Map, Value, json};

use crate::PLUGIN_ID;
use crate::config::{MAX_TEXT_CHARS, MinimaxConfig};
use crate::voices::{builtin_voices, gender_from_text, locale_of, map_emotion, pitch_to_semitones};

pub struct MinimaxTtsProvider {
    config: MinimaxConfig,
    api_key: String,
    http: reqwest::Client,
}

fn plugin_err(message: impl Into<String>) -> CoreError {
    CoreError::Plugin { plugin_id: PLUGIN_ID.into(), message: message.into() }
}

/// 十六进制字符串 → 字节
pub fn decode_hex(hex: &str) -> CoreResult<Vec<u8>> {
    let hex = hex.trim();
    if hex.len() % 2 != 0 {
        return Err(plugin_err("MiniMax 返回的音频 hex 长度不是偶数"));
    }
    let val = |c: u8| -> CoreResult<u8> {
        match c {
            b'0'..=b'9' => Ok(c - b'0'),
            b'a'..=b'f' => Ok(c - b'a' + 10),
            b'A'..=b'F' => Ok(c - b'A' + 10),
            _ => Err(plugin_err("MiniMax 返回的音频不是合法的 hex")),
        }
    };
    hex.as_bytes().chunks_exact(2).map(|p| Ok(val(p[0])? << 4 | val(p[1])?)).collect()
}

#[derive(Deserialize, Default)]
struct BaseResp {
    #[serde(default)]
    status_code: i64,
    #[serde(default)]
    status_msg: String,
}

#[derive(Deserialize)]
struct T2aResp {
    data: Option<T2aData>,
    extra_info: Option<ExtraInfo>,
    #[serde(default)]
    trace_id: String,
    #[serde(default)]
    base_resp: BaseResp,
}

#[derive(Deserialize)]
struct T2aData {
    #[serde(default)]
    audio: String,
}

#[derive(Deserialize)]
struct ExtraInfo {
    /// 音频时长，毫秒
    #[serde(default)]
    audio_length: u64,
    #[serde(default)]
    audio_sample_rate: u32,
}

#[derive(Deserialize)]
struct GetVoiceResp {
    #[serde(default)]
    system_voice: Vec<SystemVoice>,
    #[serde(default)]
    base_resp: BaseResp,
}

#[derive(Deserialize)]
struct SystemVoice {
    voice_id: String,
    #[serde(default)]
    voice_name: String,
    #[serde(default)]
    description: Vec<String>,
}

impl MinimaxTtsProvider {
    pub fn new(config: MinimaxConfig, api_key: String) -> CoreResult<Self> {
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

    /// 组装 `/v1/t2a_v2` 请求体（单独拿出来方便测试）
    pub fn build_request(&self, payload: &SynthesisPayload) -> CoreResult<Value> {
        let text = payload.text.trim();
        if text.is_empty() {
            return Err(CoreError::InvalidParameter("合成文本为空".into()));
        }
        let chars = text.chars().count();
        if chars > MAX_TEXT_CHARS {
            return Err(CoreError::InvalidParameter(format!(
                "MiniMax 同步合成单次要求少于 10000 个字符，这段有 {chars} 个，请先切句"
            )));
        }
        let voice = if payload.voice.trim().is_empty() { self.config.default_voice.as_str() } else { payload.voice.trim() };

        let mut voice_setting = Map::new();
        voice_setting.insert("voice_id".into(), json!(voice));
        voice_setting.insert("speed".into(), json!(payload.speed.clamp(0.5, 2.0)));
        voice_setting.insert("vol".into(), json!(self.config.vol));
        voice_setting.insert("pitch".into(), json!(pitch_to_semitones(payload.pitch)));
        if let Some(e) = payload.emotion.as_deref() {
            match map_emotion(e) {
                Some(v) => {
                    voice_setting.insert("emotion".into(), json!(v));
                }
                None if !e.trim().is_empty() => tracing::debug!("MiniMax 不支持情绪 {e}，交给模型自动判断"),
                None => {}
            }
        }

        let mut audio_setting = Map::new();
        audio_setting.insert("sample_rate".into(), json!(self.config.sample_rate));
        audio_setting.insert("format".into(), json!(self.config.format));
        audio_setting.insert("channel".into(), json!(self.config.channel));
        if self.config.format == "mp3" {
            audio_setting.insert("bitrate".into(), json!(self.config.bitrate));
        }

        let mut body = Map::new();
        body.insert("model".into(), json!(self.config.model));
        body.insert("text".into(), json!(text));
        body.insert("stream".into(), json!(false));
        body.insert("output_format".into(), json!("hex"));
        body.insert("voice_setting".into(), Value::Object(voice_setting));
        body.insert("audio_setting".into(), Value::Object(audio_setting));
        if let Some(b) = &self.config.language_boost {
            body.insert("language_boost".into(), json!(b));
        }
        if let Some(m) = &self.config.text_normalization_mode {
            body.insert("text_normalization_mode".into(), json!(m));
        }
        Ok(Value::Object(body))
    }

    fn audio_format(&self) -> AudioFormat {
        match self.config.format.as_str() {
            "wav" => AudioFormat::Wav,
            "flac" => AudioFormat::Flac,
            "opus" => AudioFormat::Opus,
            _ => AudioFormat::Mp3,
        }
    }

    async fn post_json(&self, path: &str, body: &Value) -> CoreResult<String> {
        let resp = self
            .http
            .post(self.url(path))
            .bearer_auth(&self.api_key)
            .json(body)
            .send()
            .await
            .map_err(|e| plugin_err(format!("请求 MiniMax {path} 失败：{e}")))?;
        let status = resp.status();
        let text = resp.text().await.map_err(|e| plugin_err(format!("读取 MiniMax 响应失败：{e}")))?;
        if !status.is_success() {
            return Err(plugin_err(format!("MiniMax {path} 返回 HTTP {status}：{}", text.chars().take(300).collect::<String>())));
        }
        Ok(text)
    }
}

#[async_trait]
impl TTSProvider for MinimaxTtsProvider {
    fn provider_id(&self) -> &str {
        crate::DRIVER_ID
    }

    async fn list_voices(&self) -> CoreResult<Vec<VoiceInfo>> {
        let result = async {
            let text = self.post_json("v1/get_voice", &json!({ "voice_type": "system" })).await?;
            let resp: GetVoiceResp =
                serde_json::from_str(&text).map_err(|e| plugin_err(format!("解析 MiniMax 音色列表失败：{e}")))?;
            if resp.base_resp.status_code != 0 {
                return Err(plugin_err(format!(
                    "MiniMax 音色列表返回错误 {}：{}",
                    resp.base_resp.status_code, resp.base_resp.status_msg
                )));
            }
            Ok(resp.system_voice)
        }
        .await;
        match result {
            Ok(list) if !list.is_empty() => Ok(list
                .into_iter()
                .map(|v| {
                    let description = v.description.join("；");
                    VoiceInfo {
                        name: if v.voice_name.is_empty() { v.voice_id.clone() } else { v.voice_name },
                        gender: gender_from_text(&description).into(),
                        locale: locale_of(&v.voice_id).into(),
                        description,
                        id: v.voice_id,
                    }
                })
                .collect()),
            Ok(_) => Ok(builtin_voices()),
            Err(e) => {
                tracing::warn!("获取 MiniMax 系统音色失败，改用内置列表：{e}");
                Ok(builtin_voices())
            }
        }
    }

    async fn synthesize(&self, payload: &SynthesisPayload) -> CoreResult<SynthesisOutput> {
        let body = self.build_request(payload)?;
        let text = self.post_json("v1/t2a_v2", &body).await?;
        let resp: T2aResp =
            serde_json::from_str(&text).map_err(|e| plugin_err(format!("解析 MiniMax 合成结果失败：{e}")))?;
        if resp.base_resp.status_code != 0 {
            return Err(plugin_err(format!(
                "MiniMax 语音合成失败 {}：{}（trace_id {}）",
                resp.base_resp.status_code, resp.base_resp.status_msg, resp.trace_id
            )));
        }
        let hex = resp.data.map(|d| d.audio).unwrap_or_default();
        if hex.is_empty() {
            return Err(plugin_err(format!("MiniMax 返回了空音频（trace_id {}）", resp.trace_id)));
        }
        let audio_bytes = decode_hex(&hex)?;
        let (duration_ms, sample_rate) = match resp.extra_info {
            Some(info) => (
                info.audio_length,
                if info.audio_sample_rate > 0 { info.audio_sample_rate } else { self.config.sample_rate },
            ),
            None => (0, self.config.sample_rate),
        };
        Ok(SynthesisOutput { audio_bytes, format: self.audio_format(), sample_rate, duration_ms })
    }
}
