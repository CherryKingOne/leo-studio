mod common;

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use axum::http::StatusCode;
use common::TestApp;
use leo_app::config::Settings;
use leo_app::storage::store::Store;
use leo_app::{build_app_state, build_router_from_state};
use leo_core::ast::AudioFormat;
use leo_core::plugin::{SynthesisOutput, SynthesisPayload, TTSProvider, VoiceInfo};
use serde_json::Value;

/// 离线假 TTS：按声线输出不同帧数的合法 MP3（MPEG-2 Layer III、24kHz、48kbps，每帧 24ms），
/// 并故意上报错误的 duration_ms，用于验证服务端按音频本身计算时长。
struct FakeTts {
    calls: AtomicUsize,
}

fn mp3_frames(count: usize) -> Vec<u8> {
    let mut frame = vec![0u8; 144];
    frame[..4].copy_from_slice(&[0xFF, 0xF3, 0x64, 0xC4]);
    frame.repeat(count)
}

#[async_trait::async_trait]
impl TTSProvider for FakeTts {
    fn provider_id(&self) -> &str {
        "fake-tts"
    }

    async fn list_voices(&self) -> leo_core::CoreResult<Vec<VoiceInfo>> {
        Ok(Vec::new())
    }

    async fn synthesize(&self, payload: &SynthesisPayload) -> leo_core::CoreResult<SynthesisOutput> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let frames = if payload.voice == "voice-b" { 25 } else { 10 };
        Ok(SynthesisOutput {
            audio_bytes: mp3_frames(frames),
            format: AudioFormat::Mp3,
            sample_rate: 24_000,
            duration_ms: 1,
        })
    }
}

async fn synthesize(app: &TestApp, url: &str, voice: &str) -> Value {
    let payload = serde_json::json!({ "voice": voice, "speed": 1.0, "pitch": 0.0, "force": false });
    let (status, body) = app.post_json(url, &payload.to_string()).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    body["data"].clone()
}

#[tokio::test]
async fn synthesis_cache_is_keyed_by_voice_and_reports_real_duration() {
    let dir = tempfile::tempdir().unwrap();
    let settings = Arc::new(Settings {
        data_dir: dir.path().to_path_buf(),
        upload_dir: dir.path().join("uploads"),
        csv_dir: dir.path().join("csv"),
        api_token: None,
        ..Settings::default()
    });
    let store = Store::new(&settings).unwrap();
    let state = build_app_state(settings, store).await.unwrap();
    let fake = Arc::new(FakeTts {
        calls: AtomicUsize::new(0),
    });
    let provider: Arc<dyn TTSProvider> = fake.clone();
    state.context.provide_named("fake-tts", provider).await.unwrap();
    state
        .context
        .set_default::<Arc<dyn TTSProvider>>("fake-tts")
        .await
        .unwrap();
    let app = TestApp {
        router: build_router_from_state(state),
    };

    let novel = "第一章 试炼开始\n言罢，他一拳轰出。\n";
    let (status, body) = app
        .post_multipart(
            "/api/v2/projects",
            &[("name", "缓存测试")],
            Some(("test.txt", "text/plain", novel.as_bytes())),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let project_id = body["data"]["project_id"].as_str().unwrap().to_string();
    let (_, body) = app.get(&format!("/api/v2/projects/{project_id}/chapters")).await;
    let chapter_id = body["data"]["items"][0]["chapter_id"].as_str().unwrap().to_string();
    let url = format!("/api/v2/projects/{project_id}/chapters/{chapter_id}/synthesize");

    // 声线 A：首次合成，时长按 10 帧 × 24ms 计算，而不是插件上报的 1ms 或 1500ms 占位
    let first = synthesize(&app, &url, "voice-a").await;
    assert_eq!(first["newly_synthesized"], 1, "{first}");
    let clip_a = first["clips"][0]["clip_id"].as_str().unwrap().to_string();
    assert_eq!(first["clips"][0]["duration_ms"], 240);
    assert_eq!(first["total_duration_ms"], 240);

    // 同一声线再次合成：命中缓存，时长从缓存音频重新计算
    let again = synthesize(&app, &url, "voice-a").await;
    assert_eq!(again["newly_synthesized"], 0, "{again}");
    assert_eq!(again["skipped_count"], 1);
    assert_eq!(again["clips"][0]["clip_id"], clip_a.as_str());
    assert_eq!(again["clips"][0]["duration_ms"], 240);
    assert_eq!(fake.calls.load(Ordering::SeqCst), 1);

    // 换声线 B（不加 force）：必须重新合成，返回新的切片而不是 A 的旧音频
    let switched = synthesize(&app, &url, "voice-b").await;
    assert_eq!(switched["newly_synthesized"], 1, "换声线后不应复用旧音频: {switched}");
    let clip_b = switched["clips"][0]["clip_id"].as_str().unwrap().to_string();
    assert_ne!(clip_a, clip_b);
    assert_eq!(switched["clips"][0]["voice"], "voice-b");
    assert_eq!(switched["clips"][0]["duration_ms"], 600);
    assert_eq!(fake.calls.load(Ordering::SeqCst), 2);

    // 两个切片各自可读，内容长度对应各自声线
    for (clip_id, frames) in [(&clip_a, 10usize), (&clip_b, 25)] {
        let (status, body) = app
            .send_raw(
                axum::http::Request::builder()
                    .uri(format!("/api/v2/audio/{clip_id}"))
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body.len(), String::from_utf8_lossy(&mp3_frames(frames)).len());
    }
}

#[test]
fn clip_id_changes_with_every_synthesis_parameter() {
    use leo_app::modules::audio::service::synthesis_clip_id;
    let base = synthesis_clip_id("s1_0", "edge-tts", "zh-CN-YunxiNeural", 1.0, 0.0, "你好");
    assert_eq!(
        base,
        synthesis_clip_id("s1_0", "edge-tts", "zh-CN-YunxiNeural", 1.0, 0.0, "你好"),
        "相同参数必须得到稳定的切片 ID"
    );
    for other in [
        synthesis_clip_id("s1_0", "other", "zh-CN-YunxiNeural", 1.0, 0.0, "你好"),
        synthesis_clip_id("s1_0", "edge-tts", "zh-CN-XiaoxiaoNeural", 1.0, 0.0, "你好"),
        synthesis_clip_id("s1_0", "edge-tts", "zh-CN-YunxiNeural", 1.5, 0.0, "你好"),
        synthesis_clip_id("s1_0", "edge-tts", "zh-CN-YunxiNeural", 1.0, 5.0, "你好"),
        synthesis_clip_id("s1_0", "edge-tts", "zh-CN-YunxiNeural", 1.0, 0.0, "您好"),
    ] {
        assert_ne!(base, other);
    }
    assert!(leo_plugin_storage_csv::is_safe_clip_id(&base));
}

#[tokio::test]
async fn synthesize_rejects_invalid_edge_parameters_with_422() {
    // 默认装配的 Edge-TTS 在本地校验参数，非法声线/语速不会联网，直接返回 422
    let app = TestApp::new(1).await;
    let novel = "第一章 试炼开始\n言罢，他一拳轰出。\n";
    let (_, body) = app
        .post_multipart(
            "/api/v2/projects",
            &[("name", "参数校验")],
            Some(("test.txt", "text/plain", novel.as_bytes())),
        )
        .await;
    let project_id = body["data"]["project_id"].as_str().unwrap().to_string();
    let (_, body) = app.get(&format!("/api/v2/projects/{project_id}/chapters")).await;
    let chapter_id = body["data"]["items"][0]["chapter_id"].as_str().unwrap().to_string();
    let url = format!("/api/v2/projects/{project_id}/chapters/{chapter_id}/synthesize");

    for payload in [
        serde_json::json!({ "voice": "zh-CN-Yunxi'><prosody rate='+500%", "speed": 1.0, "pitch": 0.0 }),
        serde_json::json!({ "voice": "zh-CN-YunxiNeural", "speed": 5.0, "pitch": 0.0 }),
        serde_json::json!({ "voice": "zh-CN-YunxiNeural", "speed": 1.0, "pitch": 500.0 }),
    ] {
        let (status, body) = app.post_json(&url, &payload.to_string()).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{payload}: {body}");
    }
}
