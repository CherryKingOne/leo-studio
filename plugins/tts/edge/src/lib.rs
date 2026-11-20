use std::sync::Arc;

use async_trait::async_trait;
use leo_core::ast::AudioFormat;
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
        ctx.provide(provider).await;
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
        let text = payload.text.clone();
        let voice_name = if payload.voice.is_empty() {
            "zh-CN-YunxiNeural".to_string()
        } else {
            payload.voice.clone()
        };

        // 在 tokio 阻塞线程池中执行 WebSocket 音频生成
        tokio::task::spawn_blocking(move || {
            let config = SpeechConfig {
                voice_name,
                audio_format: "audio-24khz-48kbitrate-mono-mp3".to_string(),
                pitch: 0,
                rate: 0,
                volume: 0,
            };

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

            // 估算时长：基于 48kbps MP3 (6000 字节/秒)
            let duration_ms = (audio.audio_bytes.len() as u64 * 1000) / 6000;

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
