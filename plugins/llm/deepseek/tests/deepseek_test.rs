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
use leo_core::spi::{ChatMessage, ChatRequest, LlmApi, LlmProvider, ResponseFormat, chat_typed};
use leo_plugin_llm_deepseek::{
    DRIVER_ID, DeepSeekConfig, DeepSeekLlmPlugin, DeepSeekLlmProvider, ReasoningEffort, is_retryable, map_http_error,
    parse_response, parse_responses_response,
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

/// 官方文档 Response 对象的形状（https://api-docs.deepseek.com/api/create-response）
fn resp_body(model: &str, text: &str) -> Value {
    json!({
        "id": "r1", "object": "response", "created_at": 1, "status": "completed", "background": false,
        "error": null, "incomplete_details": null, "model": model,
        "output": [
            {"type": "reasoning", "id": "rs", "status": "completed", "content": [{"type": "reasoning_text", "text": "思考中"}]},
            {"type": "message", "id": "m", "status": "completed", "role": "assistant", "phase": "final_answer",
             "content": [{"type": "output_text", "annotations": [], "logprobs": [], "text": text}]}
        ],
        "store": false, "text": {"format": {"type": "text"}, "verbosity": null},
        "usage": {"input_tokens": 7, "input_tokens_details": {"cached_tokens": 0}, "output_tokens": 11,
                  "output_tokens_details": {"reasoning_tokens": 0}, "total_tokens": 18}
    })
}

async fn responses(State(mock): State<Arc<Mock>>, headers: HeaderMap, Json(body): Json<Value>) -> Response {
    let auth = headers.get(header::AUTHORIZATION).and_then(|v| v.to_str().ok()).unwrap_or("").to_string();
    mock.seen.lock().unwrap().push((format!("responses {auth}"), body.clone()));
    let n = mock.calls.fetch_add(1, Ordering::SeqCst) + 1;
    match body["model"].as_str().unwrap_or("") {
        "flaky" if n <= 2 => if n == 1 { err(429, "rate limit") } else { err(503, "busy") },
        "always-500" => err(500, "boom"),
        "e400" => err(400, "invalid body"),
        "e401" => err(401, "Authentication Fails"),
        "e402" => err(402, "Insufficient Balance"),
        "e422" => err(422, "bad param"),
        "bad-schema" => Json(resp_body("bad-schema", r#"{"speaker":42}"#)).into_response(),
        m => match body["text"]["format"]["type"].as_str() {
            Some("json_object") => Json(resp_body(m, r#"{"speaker":"师姐","confidence":0.9}"#)).into_response(),
            Some("json_schema") => Json(resp_body(m, r#"{"speaker":"师姐","confidence":0.95}"#)).into_response(),
            _ => Json(resp_body(m, "你好")).into_response(),
        },
    }
}

async fn mock() -> (String, Arc<Mock>) {
    let state = Arc::new(Mock::default());
    let app = Router::new()
        .route("/chat/completions", post(chat))
        .route("/responses", post(responses))
        .with_state(state.clone());
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
        api: None,
    }
}

fn responses_provider(base_url: String) -> DeepSeekLlmProvider {
    DeepSeekLlmProvider::with_api_key(DeepSeekConfig { base_url, api: LlmApi::Responses, ..Default::default() }, "test-key")
        .unwrap()
        .with_backoff_base(Duration::from_millis(5))
}

fn line_schema() -> Value {
    json!({
        "type": "object",
        "properties": {"speaker": {"type": "string"}, "confidence": {"type": "number"}},
        "required": ["speaker", "confidence"],
        "additionalProperties": false
    })
}

// ---------- 配置 ----------

#[test]
fn config_defaults_and_parsing() {
    let c = DeepSeekConfig::from_settings(None).unwrap();
    assert_eq!(c.api_key_env, "DEEPSEEK_API_KEY");
    assert_eq!(c.base_url, "https://api.deepseek.com");
    assert_eq!(c.model, "deepseek-flash");
    assert_eq!(c.thinking, Some(false));
    assert_eq!(c.api, LlmApi::ChatCompletions, "默认保持原来的 Chat Completions 行为");
    assert_eq!(c.reasoning_effort, None);
    assert_eq!((c.timeout_secs, c.max_retries), (120, 2));

    let c = DeepSeekConfig::from_settings(Some(&json!({"api": "responses", "thinking": true, "reasoning_effort": "max"}))).unwrap();
    assert_eq!(c.api, LlmApi::Responses);
    assert_eq!(c.reasoning_effort, Some(ReasoningEffort::Max));

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
    let err = DeepSeekConfig::from_settings(Some(&json!({"api": "completions"}))).unwrap_err().to_string();
    assert!(err.contains("completions"), "{err}");
    assert!(DeepSeekConfig::from_settings(Some(&json!({"reasoning_effort": "medium", "thinking": true}))).is_err());
    let err = DeepSeekConfig::from_settings(Some(&json!({"reasoning_effort": "high"}))).unwrap_err().to_string();
    assert!(err.contains("reasoning_effort"), "thinking 默认 false 时填 reasoning_effort 应报错：{err}");
}

#[test]
fn example_config_settings_parse() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../leo-plugins.example.json");
    let v: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let c = DeepSeekConfig::from_settings(Some(&v["settings"]["leo-plugin-llm-deepseek"])).unwrap();
    assert_eq!(c.api, LlmApi::ChatCompletions);
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

    // Chat Completions 官方没有 json_schema：直接报清楚的错误，不偷偷改格式
    let err = p.build_request(&req(None).json_schema("line", line_schema(), true)).unwrap_err().to_string();
    assert!(err.contains("json_schema") && err.contains("responses"), "{err}");

    // 思考强度
    let thinking = DeepSeekLlmProvider::with_api_key(
        DeepSeekConfig { thinking: Some(true), reasoning_effort: Some(ReasoningEffort::Low), ..Default::default() },
        "k",
    )
    .unwrap();
    let body = thinking.build_request(&req(None)).unwrap();
    assert_eq!(body["thinking"], json!({"type": "enabled"}));
    assert_eq!(body["reasoning_effort"], "low");
}

#[test]
fn builds_documented_responses_body() {
    let p = responses_provider("http://unused".into());
    let (api, body) = p.build_body(&req(None)).unwrap();
    assert_eq!(api, LlmApi::Responses);
    assert_eq!(body["model"], "deepseek-flash");
    assert_eq!(body["stream"], false);
    assert_eq!(body["input"][0], json!({"type": "message", "role": "system", "content": "你是对白归属助手，输出 JSON。"}));
    assert_eq!(body["input"][1]["role"], "user");
    assert!(body.get("messages").is_none() && body.get("max_tokens").is_none() && body.get("response_format").is_none());
    assert_eq!(body["max_output_tokens"], 256);
    assert!((body["temperature"].as_f64().unwrap() - 0.2).abs() < 1e-6);
    assert_eq!(body["text"], json!({"format": {"type": "text"}}));
    assert_eq!(body["reasoning"], json!({"effort": "none"}), "thinking=false → effort none");

    let body = p.build_responses_request(&req(None).json()).unwrap();
    assert_eq!(body["text"], json!({"format": {"type": "json_object"}}));

    let body = p.build_responses_request(&req(None).json_schema("line", line_schema(), true)).unwrap();
    assert_eq!(body["text"]["format"], json!({"type": "json_schema", "name": "line", "schema": line_schema(), "strict": true}));

    assert!(p.build_responses_request(&req(None).json_schema(" ", line_schema(), true)).is_err());
    assert!(p.build_responses_request(&req(None).json_schema("x", json!("not object"), true)).is_err());
    assert!(p.build_responses_request(&ChatRequest::default()).is_err());

    // 单次请求覆盖接口
    let chat = provider("http://unused".into());
    let (api, body) = chat.build_body(&req(None).with_api(LlmApi::Responses)).unwrap();
    assert_eq!(api, LlmApi::Responses);
    assert!(body.get("input").is_some());
    let (api, _) = p.build_body(&req(None).with_api(LlmApi::ChatCompletions)).unwrap();
    assert_eq!(api, LlmApi::ChatCompletions);

    // 思考模式：thinking=null 不传 reasoning；thinking=true + effort
    let p = DeepSeekLlmProvider::with_api_key(DeepSeekConfig { thinking: None, ..Default::default() }, "k").unwrap();
    assert!(p.build_responses_request(&req(None)).unwrap().get("reasoning").is_none());
    let p = DeepSeekLlmProvider::with_api_key(
        DeepSeekConfig { thinking: Some(true), reasoning_effort: Some(ReasoningEffort::High), ..Default::default() },
        "k",
    )
    .unwrap();
    assert_eq!(p.build_responses_request(&req(None)).unwrap()["reasoning"], json!({"effort": "high"}));
}

#[test]
fn parses_documented_responses_response() {
    let r = parse_responses_response(&resp_body("deepseek-flash", "Hello").to_string()).unwrap();
    assert_eq!(r.content, "Hello", "只取 message 项的 output_text，忽略 reasoning");
    assert_eq!(r.model, "deepseek-flash");
    assert_eq!((r.usage.prompt_tokens, r.usage.completion_tokens, r.usage.total_tokens), (7, 11, 18));
    assert_eq!(r.finish_reason.as_deref(), Some("stop"));

    let mut b = resp_body("m", "{\"a\":");
    b["status"] = json!("incomplete");
    b["incomplete_details"] = json!({"reason": "max_output_tokens"});
    assert_eq!(parse_responses_response(&b.to_string()).unwrap().finish_reason.as_deref(), Some("length"));
    b["incomplete_details"] = json!({"reason": "content_filter"});
    assert_eq!(parse_responses_response(&b.to_string()).unwrap().finish_reason.as_deref(), Some("content_filter"));

    let failed = json!({"id": "r", "object": "response", "created_at": 1, "status": "failed", "model": "m", "output": [],
                        "error": {"code": "server_error", "message": "内部错误"}});
    let err = parse_responses_response(&failed.to_string()).unwrap_err().to_string();
    assert!(err.contains("failed") && err.contains("server_error") && err.contains("内部错误"), "{err}");

    let empty = json!({"id": "r", "object": "response", "created_at": 1, "status": "completed", "model": "m", "output": []});
    assert!(parse_responses_response(&empty.to_string()).unwrap_err().to_string().contains("message"));
    let in_progress = json!({"id": "r", "object": "response", "created_at": 1, "status": "in_progress", "model": "m", "output": []});
    assert!(parse_responses_response(&in_progress.to_string()).is_err());
    assert!(parse_responses_response("<html>").is_err());
}

#[test]
fn capabilities_report_both_apis() {
    let caps = provider("http://unused".into()).capabilities();
    assert_eq!(caps.default_api, LlmApi::ChatCompletions);
    let schema = ResponseFormat::json_schema("s", json!({"type": "object"}), true);
    assert!(caps.supports_format(LlmApi::ChatCompletions, &ResponseFormat::JsonObject));
    assert!(!caps.supports_format(LlmApi::ChatCompletions, &schema));
    assert!(caps.supports_format(LlmApi::Responses, &schema));
    assert_eq!(responses_provider("http://unused".into()).capabilities().default_api, LlmApi::Responses);
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

// ---------- Responses 接口（经过 HTTP） ----------

#[tokio::test]
async fn responses_text_json_object_and_json_schema() {
    let (base_url, mock) = mock().await;
    let p = responses_provider(base_url);
    let r = p.chat(req(None)).await.unwrap();
    assert_eq!(r.content, "你好");
    assert_eq!(r.usage.total_tokens, 18);
    assert_eq!(r.finish_reason.as_deref(), Some("stop"));

    let v = p.chat_json(req(None)).await.unwrap();
    assert_eq!(v["speaker"], "师姐");

    #[derive(serde::Deserialize)]
    struct Line {
        speaker: String,
        confidence: f64,
    }
    let line: Line = chat_typed(&p, req(None).json_schema("line", line_schema(), true)).await.unwrap();
    assert_eq!((line.speaker.as_str(), line.confidence), ("师姐", 0.95));

    let seen = mock.seen.lock().unwrap();
    assert_eq!(seen[0].0, "responses Bearer test-key");
    assert_eq!(seen[0].1["text"]["format"]["type"], "text");
    assert_eq!(seen[1].1["text"]["format"]["type"], "json_object");
    assert_eq!(seen[2].1["text"]["format"]["name"], "line");
    assert_eq!(seen[2].1["text"]["format"]["strict"], true);
}

#[tokio::test]
async fn per_request_api_override_hits_the_right_endpoint() {
    let (base_url, mock) = mock().await;
    let p = provider(base_url); // 配置默认 chat_completions
    p.chat(req(None).with_api(LlmApi::Responses)).await.unwrap();
    p.chat(req(None)).await.unwrap();
    let seen = mock.seen.lock().unwrap();
    assert!(seen[0].0.starts_with("responses "), "{}", seen[0].0);
    assert!(seen[1].0.starts_with("Bearer "), "{}", seen[1].0);
}

#[tokio::test]
async fn responses_schema_validation_failure_is_clear_error() {
    let (base_url, _) = mock().await;
    let err = responses_provider(base_url)
        .chat_json(req(Some("bad-schema")).json_schema("line", line_schema(), true))
        .await
        .unwrap_err()
        .to_string();
    assert!(err.contains("不符合 JSON Schema \"line\""), "{err}");
    assert!(err.contains("$.speaker 类型应为 string") && err.contains("缺少必填字段 \"confidence\""), "{err}");
}

#[tokio::test]
async fn chat_completions_rejects_json_schema_without_sending() {
    let (base_url, mock) = mock().await;
    let err = provider(base_url).chat_json(req(None).json_schema("line", line_schema(), true)).await.unwrap_err().to_string();
    assert!(err.contains("不支持 json_schema"), "{err}");
    assert_eq!(mock.calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn responses_retries_and_error_codes() {
    let (base_url, mock) = mock().await;
    let p = responses_provider(base_url);
    assert_eq!(p.chat(req(Some("flaky"))).await.unwrap().content, "你好");
    assert_eq!(mock.calls.load(Ordering::SeqCst), 3);

    let err = p.chat(req(Some("always-500"))).await.unwrap_err().to_string();
    assert!(err.contains("500") && err.contains("已重试 2 次"), "{err}");
    assert_eq!(mock.calls.load(Ordering::SeqCst), 6);

    for (model, needle) in [("e400", "请求格式错误"), ("e401", "鉴权失败"), ("e402", "余额不足"), ("e422", "参数错误")] {
        let err = p.chat(req(Some(model))).await.unwrap_err().to_string();
        assert!(err.contains(needle), "{model}: {err}");
    }
    assert_eq!(mock.calls.load(Ordering::SeqCst), 10, "4xx 不重试");
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

#[tokio::test]
#[ignore = "需要 DEEPSEEK_API_KEY，会产生少量费用"]
async fn live_responses_json_schema() {
    let p = DeepSeekLlmProvider::new(DeepSeekConfig { api: LlmApi::Responses, ..Default::default() }).unwrap();
    let schema = json!({
        "type": "object",
        "properties": {"speaker": {"type": "string"}},
        "required": ["speaker"],
        "additionalProperties": false
    });
    let req = ChatRequest {
        messages: vec![
            ChatMessage::system("判断引号里的台词是谁说的，输出 JSON。"),
            ChatMessage::user("林动看着师姐，师姐笑道：“你怎么来了？”"),
        ],
        max_tokens: Some(64),
        ..Default::default()
    }
    .json_schema("speaker", schema, true);
    let v = p.chat_json(req).await.unwrap();
    assert!(v["speaker"].is_string(), "{v}");

    let text = p.chat(ChatRequest { max_tokens: Some(16), ..ChatRequest::new(vec![ChatMessage::user("Say hello.")]) }).await.unwrap();
    assert!(!text.content.trim().is_empty() && text.usage.total_tokens > 0, "{text:?}");
}
