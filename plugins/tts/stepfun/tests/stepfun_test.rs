//! 用本地假服务器模拟阶跃星辰接口（不需要 API Key、不联网）。
//! 真实调用测试：设置 STEPFUN_API_KEY 后运行 `cargo test -p leo-plugin-tts-stepfun -- --ignored`。

use std::sync::{Arc, Mutex};

use axum::Json;
use axum::Router;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use leo_core::context::Context;
use leo_core::plugin::Plugin;
use leo_core::spi::{SynthesisPayload, TTSProvider};
use leo_plugin_tts_stepfun::{StepfunConfig, StepfunTtsPlugin, StepfunTtsProvider};
use serde_json::{Value, json};

type Seen = Arc<Mutex<Vec<(String, Value)>>>;

fn wav_bytes(ms: u32) -> Vec<u8> {
    let data_len = 48 * ms; // 24000Hz 16bit 单声道 = 48 字节/毫秒
    let mut wav = Vec::new();
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36 + data_len).to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&24000u32.to_le_bytes());
    wav.extend_from_slice(&48000u32.to_le_bytes());
    wav.extend_from_slice(&2u16.to_le_bytes());
    wav.extend_from_slice(&16u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_len.to_le_bytes());
    wav.extend(std::iter::repeat_n(0u8, data_len as usize));
    wav
}

async fn speech(State(seen): State<Seen>, headers: HeaderMap, Json(body): Json<Value>) -> impl IntoResponse {
    let auth = headers.get(header::AUTHORIZATION).and_then(|v| v.to_str().ok()).unwrap_or("").to_string();
    seen.lock().unwrap().push((auth, body.clone()));
    if body["voice"] == "bad-voice" {
        return (
            StatusCode::BAD_REQUEST,
            [(header::CONTENT_TYPE, "application/json")],
            json!({"error": {"message": "voice not found", "type": "request_params_invalid"}}).to_string().into_bytes(),
        );
    }
    (StatusCode::OK, [(header::CONTENT_TYPE, "audio/wav")], wav_bytes(750))
}

async fn system_voices() -> Json<Value> {
    Json(json!({
        "voices": ["cixingnansheng", "linjiameimei"],
        "voices-details": {
            "cixingnansheng": {"voice-name": "磁性男声", "voice-description": "男，深情厚重", "recommended_scene": "有声书"},
            "linjiameimei": {"voice-name": "邻家妹妹", "voice-description": "女，可爱亲和", "recommended_scene": "口播"}
        }
    }))
}

async fn mock() -> (String, Seen) {
    let seen: Seen = Arc::default();
    let app = Router::new()
        .route("/v1/audio/speech", post(speech))
        .route("/v1/audio/system_voices", get(system_voices))
        .with_state(seen.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    (format!("http://{addr}/v1"), seen)
}

fn payload(text: &str, voice: &str) -> SynthesisPayload {
    SynthesisPayload { text: text.into(), voice: voice.into(), speed: 1.2, pitch: 0.0, emotion: Some("高兴".into()) }
}

#[tokio::test]
async fn synthesize_sends_documented_body_and_reads_wav() {
    let (base_url, seen) = mock().await;
    let config = StepfunConfig { base_url, response_format: "wav".into(), ..Default::default() };
    let provider = StepfunTtsProvider::new(config, "test-key".into()).unwrap();

    let out = provider.synthesize(&payload("师姐，你怎么来了？", "cixingnansheng")).await.unwrap();
    assert_eq!(out.duration_ms, 750);
    assert_eq!(out.sample_rate, 24000);
    assert_eq!(out.format.extension(), "wav");

    let (auth, body) = seen.lock().unwrap()[0].clone();
    assert_eq!(auth, "Bearer test-key");
    assert_eq!(body["model"], "step-tts-2");
    assert_eq!(body["input"], "师姐，你怎么来了？");
    assert_eq!(body["voice"], "cixingnansheng");
    assert_eq!(body["response_format"], "wav");
    assert_eq!(body["sample_rate"], 24000);
    assert!((body["speed"].as_f64().unwrap() - 1.2).abs() < 1e-6);
    assert_eq!(body["voice_label"]["emotion"], "高兴");
    assert!(body.get("instruction").is_none());
}

#[tokio::test]
async fn stepaudio_models_use_instruction_instead_of_voice_label() {
    let config = StepfunConfig { model: "stepaudio-3-tts".into(), instruction: Some("说书人语气".into()), ..Default::default() };
    let provider = StepfunTtsProvider::new(config, "k".into()).unwrap();
    let body = provider.build_request(&payload("你好", "")).unwrap();
    assert_eq!(body["instruction"], "说书人语气；情绪：高兴");
    assert!(body.get("voice_label").is_none());
    assert_eq!(body["voice"], "cixingnansheng", "没指定声线时用 default_voice");
}

#[tokio::test]
async fn rejects_text_over_limit_and_surfaces_api_errors() {
    let (base_url, _) = mock().await;
    let provider = StepfunTtsProvider::new(StepfunConfig { base_url, ..Default::default() }, "k".into()).unwrap();

    let long = "字".repeat(1001);
    assert!(provider.synthesize(&payload(&long, "cixingnansheng")).await.is_err());

    let err = provider.synthesize(&payload("你好", "bad-voice")).await.unwrap_err().to_string();
    assert!(err.contains("voice not found"), "{err}");
}

#[tokio::test]
async fn list_voices_reads_official_list() {
    let (base_url, _) = mock().await;
    let provider = StepfunTtsProvider::new(StepfunConfig { base_url, ..Default::default() }, "k".into()).unwrap();
    let voices = provider.list_voices().await.unwrap();
    assert_eq!(voices.len(), 2);
    let v = voices.iter().find(|v| v.id == "linjiameimei").unwrap();
    assert_eq!(v.name, "邻家妹妹");
    assert_eq!(v.gender, "Female");
}

#[tokio::test]
async fn config_is_validated() {
    assert!(StepfunConfig::from_settings(Some(&json!({"sample_rate": 44100}))).is_err());
    assert!(StepfunConfig::from_settings(Some(&json!({"modle": "x"}))).is_err(), "写错字段名要报错");
    let c = StepfunConfig::from_settings(Some(&json!({"model": "stepaudio-3-tts"}))).unwrap();
    assert_eq!(c.model, "stepaudio-3-tts");
}

#[tokio::test]
async fn without_api_key_plugin_loads_but_registers_nothing() {
    let config = StepfunConfig { api_key_env: "LEO_TEST_STEPFUN_KEY_NOT_SET".into(), ..Default::default() };
    let ctx = Context::new("stepfun_no_key");
    StepfunTtsPlugin::new(config).apply(&ctx).await.unwrap();
    assert!(ctx.registry::<dyn TTSProvider>().get("stepfun").is_none());
}

#[tokio::test]
#[ignore = "需要 STEPFUN_API_KEY，会真实调用阶跃星辰接口并计费"]
async fn live_synthesis() {
    let ctx = Context::new("stepfun_live");
    StepfunTtsPlugin::default().apply(&ctx).await.unwrap();
    let provider = ctx.registry::<dyn TTSProvider>().get("stepfun").expect("没有 STEPFUN_API_KEY");
    let out = provider.synthesize(&payload("欢迎来到 Leo Studio。", "cixingnansheng")).await.unwrap();
    assert!(!out.audio_bytes.is_empty());
    assert!(out.duration_ms > 0);
}
