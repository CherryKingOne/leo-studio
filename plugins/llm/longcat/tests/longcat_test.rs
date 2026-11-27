//! 用本地假服务器验证协议路径 / 鉴权 / 模型过滤，不需要 API Key、不联网。
//! 真实调用测试不放进默认套件，避免 CI 花钱。

use std::sync::{Arc, Mutex};

use axum::Json;
use axum::Router;
use axum::extract::State;
use axum::http::{HeaderMap, Uri};
use axum::routing::{get, post};
use leo_core::plugin::Plugin;
use leo_core::spi::{
    ChatMessage, ChatRequest, LlmProtocol, LlmProvider, ThinkingConfig, ThinkingMode,
};
use leo_plugin_llm_longcat::{
    DEFAULT_MODEL, DEFAULT_MODELS_URL, INSERTS_THINKING_OBJECT, LongcatConfig, LongcatLlmPlugin,
    LongcatLlmProvider, SUPPORTED_PROTOCOLS,
};
use serde_json::{Value, json};

#[derive(Clone, Debug)]
struct SeenRequest {
    method: String,
    path: String,
    authorization: Option<String>,
    x_api_key: Option<String>,
    anthropic_version: Option<String>,
    body: Value,
}

type Seen = Arc<Mutex<Vec<SeenRequest>>>;

fn record(seen: &Seen, method: &str, uri: &Uri, headers: &HeaderMap, body: &Value) {
    let header = |name: &str| {
        headers
            .get(name)
            .and_then(|value| value.to_str().ok())
            .map(str::to_string)
    };
    seen.lock().unwrap().push(SeenRequest {
        method: method.to_string(),
        path: uri.path().to_string(),
        authorization: header("authorization"),
        x_api_key: header("x-api-key"),
        anthropic_version: header("anthropic-version"),
        body: body.clone(),
    });
}

async fn chat_completions(
    State(seen): State<Seen>,
    uri: Uri,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Json<Value> {
    record(&seen, "POST", &uri, &headers, &body);
    Json(json!({
        "id": "chatcmpl-test",
        "object": "chat.completion",
        "model": "mock-chat",
        "choices": [{
            "index": 0,
            "message": {"role": "assistant", "content": "聊天回答"},
            "finish_reason": "stop"
        }],
        "usage": {"prompt_tokens": 3, "completion_tokens": 2, "total_tokens": 5}
    }))
}

async fn responses(
    State(seen): State<Seen>,
    uri: Uri,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Json<Value> {
    record(&seen, "POST", &uri, &headers, &body);
    Json(json!({
        "id": "resp-test",
        "object": "response",
        "status": "completed",
        "model": "mock-responses",
        "output": [{
            "type": "message",
            "role": "assistant",
            "content": [{"type": "output_text", "text": "响应回答"}]
        }],
        "usage": {"input_tokens": 4, "output_tokens": 3, "total_tokens": 7}
    }))
}

async fn messages(
    State(seen): State<Seen>,
    uri: Uri,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Json<Value> {
    record(&seen, "POST", &uri, &headers, &body);
    Json(json!({
        "id": "msg-test",
        "type": "message",
        "role": "assistant",
        "model": "mock-anthropic",
        "content": [{"type": "text", "text": "Anthropic 回答"}],
        "stop_reason": "end_turn",
        "usage": {"input_tokens": 5, "output_tokens": 4}
    }))
}

async fn models(State(seen): State<Seen>, uri: Uri, headers: HeaderMap) -> Json<Value> {
    record(&seen, "GET", &uri, &headers, &Value::Null);
    Json(json!({
        "object": "list",
        "data": [
            {"id": format!("models/{DEFAULT_MODEL}"), "name": "本家模型"},
            {"id": "deepseek-chat", "name": "第三方模型"},
            {"id": "gpt-4o", "name": "第三方模型"}
        ]
    }))
}

async fn mock() -> (String, Seen) {
    let seen: Seen = Arc::default();
    let app = Router::new()
        .route("/chat/completions", post(chat_completions))
        .route("/responses", post(responses))
        .route("/v1/messages", post(messages))
        .route("/models", get(models))
        .with_state(seen.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    (format!("http://{addr}"), seen)
}

fn expected_path(protocol: LlmProtocol) -> &'static str {
    match protocol {
        LlmProtocol::OpenAiChat => "/chat/completions",
        LlmProtocol::OpenAiResponses => "/responses",
        LlmProtocol::Anthropic => "/v1/messages",
    }
}

fn request() -> ChatRequest {
    ChatRequest {
        messages: vec![ChatMessage::system("你是助手"), ChatMessage::user("你好")],
        temperature: Some(0.3),
        max_tokens: Some(2048),
        ..Default::default()
    }
}

#[tokio::test]
async fn supported_protocols_use_documented_endpoints_and_auth() {
    let (base_url, seen) = mock().await;
    for protocol in SUPPORTED_PROTOCOLS {
        let config = LongcatConfig {
            protocol: *protocol,
            base_url: base_url.clone(),
            stream: false,
            ..Default::default()
        };
        let provider = LongcatLlmProvider::new(config, "test-key".into()).unwrap();
        let response = provider.chat(&request()).await.unwrap();
        assert!(!response.content.is_empty());
    }

    let seen = seen.lock().unwrap();
    assert_eq!(seen.len(), SUPPORTED_PROTOCOLS.len());
    for (index, protocol) in SUPPORTED_PROTOCOLS.iter().enumerate() {
        let seen = &seen[index];
        assert_eq!(seen.method, "POST");
        assert_eq!(seen.path, expected_path(*protocol));
        if *protocol == LlmProtocol::Anthropic {
            assert_eq!(seen.x_api_key.as_deref(), Some("test-key"));
            assert_eq!(seen.anthropic_version.as_deref(), Some("2023-06-01"));
            assert!(seen.authorization.is_none());
        } else {
            assert_eq!(seen.authorization.as_deref(), Some("Bearer test-key"));
            assert!(seen.x_api_key.is_none());
        }
        assert_eq!(seen.body["model"], json!(DEFAULT_MODEL));
    }
}

#[tokio::test]
async fn remote_model_list_filters_third_party_models() {
    if DEFAULT_MODELS_URL.is_none() {
        // 讯飞 / 千帆 / 智谱公开文档没有统一 `/models`，走内置白名单。
        return;
    }
    let (base_url, seen) = mock().await;
    let config = LongcatConfig {
        base_url: base_url.clone(),
        models_url: Some(format!("{base_url}/models")),
        ..Default::default()
    };
    let provider = LongcatLlmProvider::new(config, "test-key".into()).unwrap();
    let models = provider.list_models().await.unwrap();
    assert!(models.iter().any(|model| model.id == DEFAULT_MODEL));
    assert!(!models.iter().any(|model| model.id == "deepseek-chat"));
    assert!(!models.iter().any(|model| model.id == "gpt-4o"));

    let seen = seen.lock().unwrap();
    let request = &seen[0];
    assert_eq!(request.method, "GET");
    assert_eq!(request.path, "/models");
    assert_eq!(request.authorization.as_deref(), Some("Bearer test-key"));
}

#[tokio::test]
async fn static_model_list_contains_only_owned_models() {
    if DEFAULT_MODELS_URL.is_some() {
        return;
    }
    let provider = LongcatLlmProvider::new(LongcatConfig::default(), "test-key".into()).unwrap();
    let models = provider.list_models().await.unwrap();
    assert!(models.iter().any(|model| model.id == DEFAULT_MODEL));
    assert!(!models.iter().any(|model| model.id == "deepseek-chat"));
}

#[tokio::test]
async fn thinking_is_written_to_thinking_type_when_required() {
    if !INSERTS_THINKING_OBJECT {
        return;
    }
    let (base_url, seen) = mock().await;
    let config = LongcatConfig {
        base_url,
        stream: false,
        thinking: ThinkingConfig {
            mode: ThinkingMode::Enabled,
            ..Default::default()
        },
        ..Default::default()
    };
    let provider = LongcatLlmProvider::new(config, "test-key".into()).unwrap();
    provider.chat(&request()).await.unwrap();

    let seen = seen.lock().unwrap();
    assert_eq!(seen[0].body["thinking"]["type"], json!("enabled"));
}

#[test]
fn manifest_only_declares_supported_protocols() {
    let manifest = LongcatLlmPlugin::default().manifest();
    for protocol in SUPPORTED_PROTOCOLS {
        let capability = format!("llm:protocol:{}", protocol.as_str());
        assert!(manifest.capabilities.contains(&capability));
    }
    for protocol in [
        LlmProtocol::OpenAiChat,
        LlmProtocol::OpenAiResponses,
        LlmProtocol::Anthropic,
    ] {
        if !SUPPORTED_PROTOCOLS.contains(&protocol) {
            let capability = format!("llm:protocol:{}", protocol.as_str());
            assert!(!manifest.capabilities.contains(&capability));
        }
    }
}
