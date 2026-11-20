use std::sync::Arc;

use async_trait::async_trait;
use leo_core::ast::{AudioFormat, mp3_duration_ms};
use leo_core::context::Context;
use leo_core::error::{CoreError, CoreResult};
use leo_core::plugin::{
    Plugin, PluginCategory, PluginManifest, SynthesisOutput, SynthesisPayload, TTSProvider,
    VoiceInfo,
};
use msedge_tts::tts::client::connect;
use msedge_tts::tts::SpeechConfig;

pub struct EdgeTtsPlugin;

impl Default for EdgeTtsPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl EdgeTtsPlugin {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Plugin for EdgeTtsPlugin {
    fn manifest(&self) -> PluginManifest {
        let mut manifest = PluginManifest::new(
            "leo-plugin-tts-edge",
            "微软 Edge-TTS 语音合成",
            "0.1.0",
            PluginCategory::Tts,
        );
        manifest.description =
            "基于微软 Edge 神经语音的免费高品质 TTS 插件，提供云希、晓晓等多种小说声线。".into();
        manifest.capabilities = vec!["tts:synthesize".into(), "tts:list_voices".into()];
        manifest
    }

    async fn apply(&self, ctx: &Context) -> CoreResult<()> {
        let provider: Arc<dyn TTSProvider> = Arc::new(EdgeTtsProvider::new());
        ctx.provide_named("edge-tts", provider).await?;
        tracing::info!("Edge-TTS 插件加载完成，已注册 TTSProvider 服务");
        Ok(())
    }
}

pub struct EdgeTtsProvider {
    builtin_voices: Vec<VoiceInfo>,
}

impl Default for EdgeTtsProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl EdgeTtsProvider {
    pub fn new() -> Self {
        Self {
            builtin_voices: vec![
                VoiceInfo {
                    id: "zh-CN-YunxiNeural".into(),
                    name: "云希 (热门玄幻/旁白/青年男声)".into(),
                    gender: "Male".into(),
                    locale: "zh-CN".into(),
                    description: "沉稳大气，极适合玄幻修仙正文旁白与男主配音".into(),
                },
                VoiceInfo {
                    id: "zh-CN-XiaoxiaoNeural".into(),
                    name: "晓晓 (温柔知性/女主配音)".into(),
                    gender: "Female".into(),
                    locale: "zh-CN".into(),
                    description: "温婉细腻，极适合言情女主与灵动师姐角色".into(),
                },
                VoiceInfo {
                    id: "zh-CN-YunjianNeural".into(),
                    name: "云健 (武侠大叔/宗门长辈)".into(),
                    gender: "Male".into(),
                    locale: "zh-CN".into(),
                    description: "中气十足，威严庄重，适合宗主、反派高手、中老年角色".into(),
                },
                VoiceInfo {
                    id: "zh-CN-XiaoyiNeural".into(),
                    name: "晓伊 (活泼少女/小师妹)".into(),
                    gender: "Female".into(),
                    locale: "zh-CN".into(),
                    description: "清脆欢快，适合傲娇萝莉、年轻师妹角色".into(),
                },
            ],
        }
    }
}

#[async_trait]
impl TTSProvider for EdgeTtsProvider {
    fn provider_id(&self) -> &str {
        "edge-tts"
    }

    async fn list_voices(&self) -> CoreResult<Vec<VoiceInfo>> {
        // 先返回预置的精选小说高频中文声线
        Ok(self.builtin_voices.clone())
    }

    async fn synthesize(&self, payload: &SynthesisPayload) -> CoreResult<SynthesisOutput> {
        // 先在本地校验并换算参数，非法输入不会发起任何网络请求
        let config = speech_config(payload)?;
        let text = escape_ssml_text(&payload.text);

        // 在 tokio 阻塞线程池中执行 WebSocket 音频生成
        tokio::task::spawn_blocking(move || {
            let mut client = connect().map_err(|e| {
                CoreError::Plugin {
                    plugin_id: "leo-plugin-tts-edge".into(),
                    message: format!("连接 Edge-TTS 服务失败: {e}"),
                }
            })?;

            let audio = client.synthesize(&text, &config).map_err(|e| {
                CoreError::Plugin {
                    plugin_id: "leo-plugin-tts-edge".into(),
                    message: format!("语音合成失败: {e}"),
                }
            })?;

            // 按 MP3 帧头计算真实时长；无法解析时退回 48kbps (6000 字节/秒) 估算
            let duration_ms = mp3_duration_ms(&audio.audio_bytes)
                .unwrap_or((audio.audio_bytes.len() as u64 * 1000) / 6000);

            Ok(SynthesisOutput {
                audio_bytes: audio.audio_bytes,
                format: AudioFormat::Mp3,
                sample_rate: 24000,
                duration_ms,
            })
        })
        .await
        .map_err(|join_err| CoreError::Internal(format!("TTS 任务执行异常: {join_err}")))?
    }
}

/// 默认声线：未指定 voice 时使用
pub const DEFAULT_VOICE: &str = "zh-CN-YunxiNeural";
/// 允许的语速倍率范围
pub const SPEED_RANGE: (f32, f32) = (0.5, 2.0);
/// 允许的音调偏移范围（Hz）
pub const PITCH_RANGE_HZ: (f32, f32) = (-100.0, 100.0);

fn invalid(message: String) -> CoreError {
    CoreError::InvalidParameter(message)
}

/// 把通用合成参数换算为 Edge-TTS 的 SSML prosody 配置。
///
/// - `speed` 是语速倍率（0.5 ~ 2.0，1.0 为原速），换算为 `rate` 百分比：`(speed - 1) × 100`，
///   例如 1.5 → `+50%`、0.8 → `-20%`；
/// - `pitch` 是音调偏移，单位 Hz（-100 ~ +100，0 为原调），直接写入 `pitch='±NHz'`；
/// - `voice` 只允许 ASCII 字母、数字与 `-`（如 `zh-CN-YunxiNeural`），防止把引号等字符注入 SSML 属性。
///
/// 超出范围或非有限数值返回 `CoreError::InvalidParameter`。
pub fn speech_config(payload: &SynthesisPayload) -> CoreResult<SpeechConfig> {
    let voice_name = if payload.voice.trim().is_empty() {
        DEFAULT_VOICE.to_string()
    } else {
        payload.voice.trim().to_string()
    };
    if voice_name.len() > 64
        || !voice_name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-')
    {
        return Err(invalid(format!(
            "非法的声线 ID {voice_name:?}：只允许字母、数字与 '-'，如 {DEFAULT_VOICE}"
        )));
    }

    let speed = payload.speed;
    if !speed.is_finite() || speed < SPEED_RANGE.0 || speed > SPEED_RANGE.1 {
        return Err(invalid(format!(
            "语速倍率 speed={speed} 超出范围 {} ~ {}",
            SPEED_RANGE.0, SPEED_RANGE.1
        )));
    }
    let pitch = payload.pitch;
    if !pitch.is_finite() || pitch < PITCH_RANGE_HZ.0 || pitch > PITCH_RANGE_HZ.1 {
        return Err(invalid(format!(
            "音调偏移 pitch={pitch}Hz 超出范围 {} ~ {}",
            PITCH_RANGE_HZ.0, PITCH_RANGE_HZ.1
        )));
    }

    Ok(SpeechConfig {
        voice_name,
        audio_format: "audio-24khz-48kbitrate-mono-mp3".to_string(),
        pitch: pitch.round() as i32,
        rate: ((speed - 1.0) * 100.0).round() as i32,
        volume: 0,
    })
}

/// 对要放进 SSML 的正文做 XML 转义（`&`、`<`、`>`、`"`、`'`），
/// 并去掉 XML 1.0 不允许的控制字符，避免生成非法 SSML 导致合成失败或被截断。
pub fn escape_ssml_text(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&apos;"),
            '\t' | '\n' | '\r' => escaped.push(ch),
            c if (c as u32) < 0x20 => {}
            c => escaped.push(c),
        }
    }
    escaped
}
