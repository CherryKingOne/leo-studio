//! 用本地假服务器模拟 DeepSeek 接口（不需要 API Key、不联网）。
//! 真实调用测试：设置 DEEPSEEK_API_KEY 后运行 `cargo test -p leo-plugin-llm-deepseek -- --ignored`（会计费）。

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::Json;
use axum::Router;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use leo_core::context::Context;
use leo_core::plugin::Plugin;
use leo_core::spi::{ChatMessage, ChatRequest, LlmProvider, ResponseFormat};
use leo_plugin_llm_deepseek::{
    DRIVER_ID, DeepSeekConfig, DeepSeekLlmPlugin, DeepSeekLlmProvider, is_retryable, map_http_error, parse_response,
};
use serde_json::{Value, json};

#[derive(Default)]
struct Mock {
    seen: Mutex<Vec<(String, Value)>>,
    calls: AtomicU32,
}

fn ok_body(model: &str, content: &str) -> Value {
    json!({
        "id": "x", "object": "chat.completion", "created": 1, "model": model,
        "choices": [{"index": 0, "message": {"role": "assistant", "content": content}, "logprobs": null, "finish_reason": "stop"}],
        "usage": {"prompt_tokens": 7, "completion_tokens": 11, "total_tokens": 18, "prompt_cache_hit_tokens": 0, "prompt_cache_miss_tokens": 7},
        "system_fingerprint": "fp"
    })
}

fn err(status: u16, message: &str) -> Response {
    let body = json!({"error": {"message": message, "type": "invalid_request_error", "param": null, "code": null}});
    (StatusCode::from_u16(status).unwrap(), Json(body)).into_response()
}

async fn chat(State(mock): State<Arc<Mock>>, headers: HeaderMap, Json(body): Json<Value>) -> Response {
    let auth = headers.get(header::AUTHORIZATION).and_then(|v| v.to_str().ok()).unwrap_or("").to_string();
    mock.seen.lock().unwrap().push((auth, body.clone()));
    let n = mock.calls.fetch_add(1, Ordering::SeqCst) + 1;
    match body["model"].as_str().unwrap_or("") {
        "flaky" if n <= 2 => if n == 1 { err(429, "rate limit") } else { err(500, "boom") },
        "always-503" => err(503, "overloaded"),
        "e400" => err(400, "invalid body"),
        "e401" => err(401, "Authentication Fails"),
        "e402" => err(402, "Insufficient Balance"),
        "e422" => err(422, "max_tokens out of range"),
        "bad-json" => Json(ok_body("bad-json", "这不是 JSON")).into_response(),
        m if body["response_format"]["type"] == "json_object" => {
            Json(ok_body(m, r#"{"speaker":"师姐","confidence":0.9}"#)).into_response()
        }
        m => Json(ok_body(m, "你好")).into_response(),
    }
}

async fn mock() -> (String, Arc<Mock>) {
    let state = Arc::new(Mock::default());
    let app = Router::new().route("/chat/completions", post(chat)).with_state(state.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    (format!("http://{addr}"), state)
}

fn provider(base_url: String) -> DeepSeekLlmProvider {
    DeepSeekLlmProvider::with_api_key(DeepSeekConfig { base_url, ..Default::default() }, "test-key")
        .unwrap()
        .with_backoff_base(Duration::from_millis(5))
}

fn req(model: Option<&str>) -> ChatRequest {
    ChatRequest {
        model: model.map(Into::into),
        messages: vec![ChatMessage::system("你是对白归属助手，输出 JSON。"), ChatMessage::user("“师姐，你怎么来了？”")],
        temperature: Some(0.2),
        max_tokens: Some(256),
        response_format: ResponseFormat::Text,
    }
}

// ---------- 配置 ----------

#[test]
fn config_defaults_and_parsing() {
    let c = DeepSeekConfig::from_settings(None).unwrap();
    assert_eq!(c.api_key_env, "DEEPSEEK_API_KEY");
    assert_eq!(c.base_url, "https://api.deepseek.com");
    assert_eq!(c.model, "deepseek-flash");
    assert_eq!(c.thinking, Some(false));
    assert_eq!((c.timeout_secs, c.max_retries), (120, 2));

    let v = json!({"model": "deepseek-v4-pro", "temperature": 0.3, "max_tokens": 2048, "thinking": null, "max_retries": 0});
    let c = DeepSeekConfig::from_settings(Some(&v)).unwrap();
    assert_eq!(c.model, "deepseek-v4-pro");
    assert_eq!(c.temperature, Some(0.3));
    assert_eq!(c.max_tokens, Some(2048));
    assert_eq!(c.thinking, None);
    assert_eq!(c.max_retries, 0);
}

#[test]
fn config_rejects_unknown_fields_and_bad_values() {
    let err = DeepSeekConfig::from_settings(Some(&json!({"modle": "x"}))).unwrap_err().to_string();
    assert!(err.contains("modle"), "{err}");
    assert!(DeepSeekConfig::from_settings(Some(&json!({"api_key": "sk-xxx"}))).is_err());
    let err = DeepSeekConfig::from_settings(Some(&json!({"temperature": 3.0}))).unwrap_err().to_string();
    assert!(err.contains("temperature"), "{err}");
    assert!(DeepSeekConfig::from_settings(Some(&json!({"max_tokens": 0}))).is_err());
    assert!(DeepSeekConfig::from_settings(Some(&json!({"base_url": "api.deepseek.com"}))).is_err());
    assert!(DeepSeekConfig::from_settings(Some(&json!({"timeout_secs": 0}))).is_err());
    assert!(DeepSeekConfig::from_settings(Some(&json!({"max_retries": 99}))).is_err());
}

// ---------- 请求体与响应解析 ----------

#[test]
fn builds_documented_request_body() {
    let p = provider("http://unused".into());
    let body = p.build_request(&req(None)).unwrap();
    assert_eq!(body["model"], "deepseek-flash");
    assert_eq!(body["stream"], false);
    assert_eq!(body["messages"][0], json!({"role": "system", "content": "你是对白归属助手，输出 JSON。"}));
    assert_eq!(body["messages"][1]["role"], "user");
    assert!((body["temperature"].as_f64().unwrap() - 0.2).abs() < 1e-6);
    assert_eq!(body["max_tokens"], 256);
    assert_eq!(body["response_format"], json!({"type": "text"}));
    assert_eq!(body["thinking"], json!({"type": "disabled"}));

    let body = p.build_request(&ChatRequest { model: Some("deepseek-v4-pro".into()), ..req(None) }.json()).unwrap();
    assert_eq!(body["model"], "deepseek-v4-pro");
    assert_eq!(body["response_format"], json!({"type": "json_object"}));

    let bare = p.build_request(&ChatRequest::new(vec![ChatMessage::user("hi")])).unwrap();
    assert!(bare.get("temperature").is_none() && bare.get("max_tokens").is_none());

    assert!(p.build_request(&ChatRequest::default()).is_err());
    assert!(p.build_request(&ChatRequest { temperature: Some(-1.0), ..req(None) }).is_err());
}

#[test]
fn parses_documented_response() {
    let r = parse_response(&ok_body("deepseek-flash", "Hello").to_string()).unwrap();
    assert_eq!(r.content, "Hello");
    assert_eq!(r.model, "deepseek-flash");
    assert_eq!((r.usage.prompt_tokens, r.usage.completion_tokens, r.usage.total_tokens), (7, 11, 18));
    assert_eq!(r.finish_reason.as_deref(), Some("stop"));

    let null_content = json!({"model": "m", "choices": [{"index": 0, "message": {"role": "assistant", "content": null}, "finish_reason": "length"}]});
    let r = parse_response(&null_content.to_string()).unwrap();
    assert_eq!(r.content, "");
    assert_eq!(r.usage.total_tokens, 0);

    assert!(parse_response(r#"{"model":"m","choices":[]}"#).unwrap_err().to_string().contains("choices"));
    assert!(parse_response("<html>").is_err());
}

#[test]
fn error_mapping() {
    assert!(is_retryable(429) && is_retryable(500) && is_retryable(503));
    assert!(!is_retryable(400) && !is_retryable(401) && !is_retryable(402) && !is_retryable(422));
    let msg = |s, m| map_http_error(s, &json!({"error": {"message": m}}).to_string()).to_string();
    assert!(msg(401, "Authentication Fails").contains("API Key"));
    assert!(msg(402, "Insufficient Balance").contains("余额不足"));
    assert!(msg(400, "bad").contains("请求格式错误"));
    assert!(msg(422, "bad param").contains("参数错误") && msg(422, "bad param").contains("bad param"));
    assert!(map_http_error(502, "upstream down").to_string().contains("upstream down"));
}

// ---------- 经过 HTTP 的调用 ----------

#[tokio::test]
async fn chat_sends_bearer_and_parses() {
    let (base_url, mock) = mock().await;
    let p = provider(base_url);
    let r = p.chat(req(None)).await.unwrap();
    assert_eq!(r.content, "你好");
    assert_eq!(r.usage.total_tokens, 18);
    let (auth, body) = mock.seen.lock().unwrap()[0].clone();
    assert_eq!(auth, "Bearer test-key");
    assert_eq!(body["model"], "deepseek-flash");

    let v = p.chat_json(req(None)).await.unwrap();
    assert_eq!(v["speaker"], "师姐");
    assert_eq!(mock.seen.lock().unwrap()[1].1["response_format"]["type"], "json_object");
}

#[tokio::test]
async fn chat_json_invalid_content_is_clear_error() {
    let (base_url, _) = mock().await;
    let err = provider(base_url).chat_json(req(Some("bad-json"))).await.unwrap_err().to_string();
    assert!(err.contains("不是合法 JSON"), "{err}");
}

#[tokio::test]
async fn retries_429_and_5xx_then_succeeds() {
    let (base_url, mock) = mock().await;
    let r = provider(base_url).chat(req(Some("flaky"))).await.unwrap();
    assert_eq!(r.content, "你好");
    assert_eq!(mock.calls.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn gives_up_after_max_retries() {
    let (base_url, mock) = mock().await;
    let err = provider(base_url).chat(req(Some("always-503"))).await.unwrap_err().to_string();
    assert!(err.contains("503") && err.contains("已重试 2 次"), "{err}");
    assert_eq!(mock.calls.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn client_errors_are_not_retried() {
    let (base_url, mock) = mock().await;
    let p = provider(base_url);
    for (model, needle) in [("e400", "请求格式错误"), ("e401", "鉴权失败"), ("e402", "余额不足"), ("e422", "参数错误")] {
        let err = p.chat(req(Some(model))).await.unwrap_err().to_string();
        assert!(err.contains(needle), "{model}: {err}");
    }
    assert_eq!(mock.calls.load(Ordering::SeqCst), 4);
}

#[tokio::test]
async fn missing_api_key_errors_at_call_time() {
    let (base_url, mock) = mock().await;
    let config = DeepSeekConfig { base_url, api_key_env: "LEO_TEST_DEEPSEEK_KEY_NOT_SET".into(), ..Default::default() };
    let p = DeepSeekLlmProvider::new(config).unwrap();
    let err = p.chat(req(None)).await.unwrap_err().to_string();
    assert!(err.contains("LEO_TEST_DEEPSEEK_KEY_NOT_SET"), "{err}");
    assert_eq!(mock.calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn plugin_registers_driver_without_key() {
    let ctx = Context::new("t");
    let config = DeepSeekConfig { api_key_env: "LEO_TEST_DEEPSEEK_KEY_NOT_SET".into(), ..Default::default() };
    DeepSeekLlmPlugin::new(config).apply(&ctx.for_plugin(leo_plugin_llm_deepseek::PLUGIN_ID)).await.unwrap();
    let registry = ctx.registry::<dyn LlmProvider>();
    assert_eq!(registry.get(DRIVER_ID).unwrap().provider_id(), "deepseek");
}

// ---------- 真实接口 ----------

#[tokio::test]
#[ignore = "需要 DEEPSEEK_API_KEY，会产生少量费用"]
async fn live_chat_json() {
    let p = DeepSeekLlmProvider::new(DeepSeekConfig::default()).unwrap();
    let req = ChatRequest {
        messages: vec![
            ChatMessage::system("判断引号里的台词是谁说的，只输出 JSON，格式：{\"speaker\": \"人名\"}"),
            ChatMessage::user("林动看着师姐，师姐笑道：“你怎么来了？”"),
        ],
        max_tokens: Some(64),
        ..Default::default()
    };
    let v = p.chat_json(req).await.unwrap();
    assert!(v.get("speaker").is_some(), "{v}");
}
