//! 用本地假服务器模拟 MiniMax 接口（不需要 API Key、不联网）。
//! 真实调用测试：设置 MINIMAX_API_KEY 后运行 `cargo test -p leo-plugin-tts-minimax -- --ignored`。

use std::sync::{Arc, Mutex};

use axum::Json;
use axum::Router;
use axum::extract::State;
use axum::http::{HeaderMap, header};
use axum::routing::post;
use leo_core::context::Context;
use leo_core::plugin::Plugin;
use leo_core::spi::{SynthesisPayload, TTSProvider};
use leo_plugin_tts_minimax::{MinimaxConfig, MinimaxTtsPlugin, MinimaxTtsProvider, decode_hex, map_emotion, pitch_to_semitones};
use serde_json::{Value, json};

type Seen = Arc<Mutex<Vec<(String, Value)>>>;

async fn t2a(State(seen): State<Seen>, headers: HeaderMap, Json(body): Json<Value>) -> Json<Value> {
    let auth = headers.get(header::AUTHORIZATION).and_then(|v| v.to_str().ok()).unwrap_or("").to_string();
    seen.lock().unwrap().push((auth, body.clone()));
    if body["voice_setting"]["voice_id"] == "bad-voice" {
        // MiniMax 的业务错误：HTTP 200 + base_resp.status_code 非 0
        return Json(json!({"trace_id": "t-err", "base_resp": {"status_code": 2054, "status_msg": "voice id not exist"}}));
    }
    Json(json!({
        "data": {"audio": "49443303deadbeef", "status": 2},
        "extra_info": {"audio_length": 1234, "audio_sample_rate": 32000, "audio_size": 8, "audio_format": "mp3", "audio_channel": 1},
        "trace_id": "t-ok",
        "base_resp": {"status_code": 0, "status_msg": "success"}
    }))
}

async fn get_voice() -> Json<Value> {
    Json(json!({
        "system_voice": [
            {"voice_id": "Chinese (Mandarin)_News_Anchor", "voice_name": "新闻女声", "description": ["一位专业、播音腔的中年女性新闻主播，标准普通话。"]},
            {"voice_id": "English_Graceful_Lady", "voice_name": "Graceful Lady", "description": []}
        ],
        "base_resp": {"status_code": 0, "status_msg": "success"}
    }))
}

async fn mock() -> (String, Seen) {
    let seen: Seen = Arc::default();
    let app = Router::new()
        .route("/v1/t2a_v2", post(t2a))
        .route("/v1/get_voice", post(get_voice))
        .with_state(seen.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    (format!("http://{addr}"), seen)
}

fn payload(voice: &str) -> SynthesisPayload {
    SynthesisPayload { text: "师姐，你怎么来了？".into(), voice: voice.into(), speed: 0.9, pitch: 50.0, emotion: Some("惊讶".into()) }
}

#[tokio::test]
async fn synthesize_sends_documented_body_and_decodes_hex() {
    let (base_url, seen) = mock().await;
    let provider = MinimaxTtsProvider::new(MinimaxConfig { base_url, ..Default::default() }, "test-key".into()).unwrap();

    let out = provider.synthesize(&payload("Chinese (Mandarin)_News_Anchor")).await.unwrap();
    assert_eq!(out.audio_bytes, vec![0x49, 0x44, 0x33, 0x03, 0xde, 0xad, 0xbe, 0xef]);
    assert_eq!(out.duration_ms, 1234);
    assert_eq!(out.sample_rate, 32000);
    assert_eq!(out.format.extension(), "mp3");

    let (auth, body) = seen.lock().unwrap()[0].clone();
    assert_eq!(auth, "Bearer test-key");
    assert_eq!(body["model"], "speech-2.8-hd");
    assert_eq!(body["stream"], false);
    assert_eq!(body["output_format"], "hex");
    assert_eq!(body["voice_setting"]["voice_id"], "Chinese (Mandarin)_News_Anchor");
    assert_eq!(body["voice_setting"]["pitch"], 6);
    assert_eq!(body["voice_setting"]["emotion"], "surprised");
    assert!((body["voice_setting"]["speed"].as_f64().unwrap() - 0.9).abs() < 1e-6);
    assert_eq!(body["audio_setting"]["format"], "mp3");
    assert_eq!(body["audio_setting"]["bitrate"], 128000);
    assert_eq!(body["audio_setting"]["sample_rate"], 32000);
}

#[tokio::test]
async fn business_error_in_base_resp_is_reported() {
    let (base_url, _) = mock().await;
    let provider = MinimaxTtsProvider::new(MinimaxConfig { base_url, ..Default::default() }, "k".into()).unwrap();
    let err = provider.synthesize(&payload("bad-voice")).await.unwrap_err().to_string();
    assert!(err.contains("2054") && err.contains("voice id not exist"), "{err}");
}

#[tokio::test]
async fn list_voices_reads_system_voices() {
    let (base_url, _) = mock().await;
    let provider = MinimaxTtsProvider::new(MinimaxConfig { base_url, ..Default::default() }, "k".into()).unwrap();
    let voices = provider.list_voices().await.unwrap();
    assert_eq!(voices.len(), 2);
    let anchor = voices.iter().find(|v| v.id == "Chinese (Mandarin)_News_Anchor").unwrap();
    assert_eq!(anchor.name, "新闻女声");
    assert_eq!(anchor.gender, "Female");
    assert_eq!(anchor.locale, "zh-CN");
}

#[test]
fn helpers() {
    assert_eq!(decode_hex("00ff10").unwrap(), vec![0, 255, 16]);
    assert!(decode_hex("abc").is_err());
    assert_eq!(map_emotion("生气"), Some("angry"));
    assert_eq!(map_emotion("whisper"), Some("whisper"));
    assert_eq!(map_emotion("阴阳怪气"), None);
    assert_eq!(pitch_to_semitones(-100.0), -12);
    assert_eq!(pitch_to_semitones(0.0), 0);
    assert_eq!(pitch_to_semitones(300.0), 12);
}

#[test]
fn config_is_validated() {
    assert!(MinimaxConfig::from_settings(Some(&json!({"bitrate": 1000}))).is_err());
    assert!(MinimaxConfig::from_settings(Some(&json!({"vol": 0}))).is_err());
    assert!(MinimaxConfig::from_settings(Some(&json!({"modle": "x"}))).is_err(), "写错字段名要报错");
    let c = MinimaxConfig::from_settings(Some(&json!({"model": "speech-2.6-turbo", "format": "wav"}))).unwrap();
    assert_eq!(c.model, "speech-2.6-turbo");
}

#[tokio::test]
async fn without_api_key_plugin_loads_but_registers_nothing() {
    let config = MinimaxConfig { api_key_env: "LEO_TEST_MINIMAX_KEY_NOT_SET".into(), ..Default::default() };
    let ctx = Context::new("minimax_no_key");
    MinimaxTtsPlugin::new(config).apply(&ctx).await.unwrap();
    assert!(ctx.registry::<dyn TTSProvider>().get("minimax").is_none());
}

#[tokio::test]
#[ignore = "需要 MINIMAX_API_KEY，会真实调用 MiniMax 接口并计费"]
async fn live_synthesis() {
    let ctx = Context::new("minimax_live");
    MinimaxTtsPlugin::default().apply(&ctx).await.unwrap();
    let provider = ctx.registry::<dyn TTSProvider>().get("minimax").expect("没有 MINIMAX_API_KEY");
    let out = provider.synthesize(&payload("male-qn-qingse")).await.unwrap();
    assert!(!out.audio_bytes.is_empty());
    assert!(out.duration_ms > 0);
}
