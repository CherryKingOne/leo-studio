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
