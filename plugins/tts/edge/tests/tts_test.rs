use std::sync::Arc;

use leo_core::context::Context;
use leo_core::plugin::{Plugin, SynthesisPayload, TTSProvider};
use leo_plugin_tts_edge::EdgeTtsPlugin;

#[tokio::test]
async fn test_edge_tts_plugin_registration() {
    let ctx = Context::new("test_project");
    let plugin = EdgeTtsPlugin::new();

    // 验证清单
    let manifest = plugin.manifest();
    assert_eq!(manifest.id, "leo-plugin-tts-edge");
    assert!(manifest.capabilities.contains(&"tts:synthesize".to_string()));

    // 挂载插件
    plugin.apply(&ctx).await.expect("插件加载失败");

    // 依赖注入提取 TTS 服务
    let provider = ctx
        .inject::<Arc<dyn TTSProvider>>()
        .await
        .expect("未能获取 TTSProvider 服务");

    // 验证声线列表
    let voices = provider.list_voices().await.expect("获取声线失败");
    assert!(voices.iter().any(|v| v.id == "zh-CN-YunxiNeural"));
    assert!(voices.iter().any(|v| v.id == "zh-CN-XiaoxiaoNeural"));
}

#[tokio::test]
async fn test_edge_tts_live_synthesis() {
    let plugin = EdgeTtsPlugin::new();
    let ctx = Context::new("test_live_synthesis");
    plugin.apply(&ctx).await.unwrap();

    let provider = ctx.inject::<Arc<dyn TTSProvider>>().await.unwrap();

    let payload = SynthesisPayload {
        text: "欢迎来到 Leo Studio，有声书插件化引擎已成功启动。".into(),
        voice: "zh-CN-YunxiNeural".into(),
        speed: 1.0,
        pitch: 0.0,
        emotion: None,
    };

    // 尝试调用在线合成
    match provider.synthesize(&payload).await {
        Ok(output) => {
            println!("成功生成音频！大小：{} 字节，格式：{:?}，预估时长：{}ms", 
                output.audio_bytes.len(), 
                output.format, 
                output.duration_ms
            );
            assert!(!output.audio_bytes.is_empty());
            assert!(output.audio_bytes.len() > 1000);
        }
        Err(e) => {
            // 在受限网络环境下打印警告，不中断离线测试
            eprintln!("网络环境无法连接 Edge-TTS 服务器（可能是离线环境）：{e}");
        }
    }
}

fn payload(text: &str, voice: &str, speed: f32, pitch: f32) -> SynthesisPayload {
    SynthesisPayload {
        text: text.into(),
        voice: voice.into(),
        speed,
        pitch,
        emotion: None,
    }
}

#[test]
fn test_speed_and_pitch_are_mapped_to_prosody() {
    use leo_plugin_tts_edge::speech_config;

    let config = speech_config(&payload("你好", "zh-CN-XiaoxiaoNeural", 1.5, -20.0)).unwrap();
    assert_eq!(config.voice_name, "zh-CN-XiaoxiaoNeural");
    assert_eq!(config.rate, 50);
    assert_eq!(config.pitch, -20);

    let config = speech_config(&payload("你好", "", 0.8, 12.4)).unwrap();
    assert_eq!(config.voice_name, "zh-CN-YunxiNeural");
    assert_eq!(config.rate, -20);
    assert_eq!(config.pitch, 12);

    let config = speech_config(&payload("你好", "zh-CN-YunxiNeural", 1.0, 0.0)).unwrap();
    assert_eq!((config.rate, config.pitch), (0, 0));

    for (speed, pitch) in [(0.4, 0.0), (2.1, 0.0), (f32::NAN, 0.0), (1.0, 101.0), (1.0, f32::INFINITY)] {
        assert!(
            speech_config(&payload("你好", "zh-CN-YunxiNeural", speed, pitch)).is_err(),
            "speed={speed} pitch={pitch} 应被拒绝"
        );
    }
}

#[test]
fn test_ssml_text_and_voice_are_escaped_or_rejected() {
    use leo_plugin_tts_edge::{escape_ssml_text, speech_config};

    assert_eq!(
        escape_ssml_text("A&B <i>\"嗨\"</i> 'x'"),
        "A&amp;B &lt;i&gt;&quot;嗨&quot;&lt;/i&gt; &apos;x&apos;"
    );
    assert_eq!(escape_ssml_text("第一句\u{0}\u{8}结束\n"), "第一句结束\n");
    assert_eq!(escape_ssml_text("&amp;"), "&amp;amp;");

    for voice in ["zh-CN-Yunxi'><prosody rate='+500%", "zh CN", "a\"b", "云希"] {
        assert!(speech_config(&payload("你好", voice, 1.0, 0.0)).is_err(), "{voice}");
    }
}

#[tokio::test]
async fn test_invalid_parameters_fail_before_network() {
    use leo_core::error::CoreError;
    use leo_plugin_tts_edge::EdgeTtsProvider;

    let provider = EdgeTtsProvider::new();
    let err = provider
        .synthesize(&payload("你好", "bad'voice", 1.0, 0.0))
        .await
        .expect_err("非法声线应直接报错");
    assert!(matches!(err, CoreError::InvalidParameter(_)), "{err}");
    let err = provider
        .synthesize(&payload("你好", "zh-CN-YunxiNeural", 3.0, 0.0))
        .await
        .expect_err("越界语速应直接报错");
    assert!(matches!(err, CoreError::InvalidParameter(_)), "{err}");
}
