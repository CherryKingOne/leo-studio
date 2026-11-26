//! 用本地假服务器模拟 DeepSeek 的三种协议入口（不需要 API Key、不联网）。
//! 真实调用测试：设置 DEEPSEEK_API_KEY 后运行 `cargo test -p leo-plugin-llm-deepseek -- --ignored`。

use std::sync::{Arc, Mutex};

use axum::Json;
use axum::Router;
use axum::extract::State;
use axum::http::{HeaderMap, Uri};
use axum::routing::post;
use leo_core::context::Context;
use leo_core::plugin::Plugin;
use leo_core::spi::{ChatMessage, ChatRequest, LlmProvider};
use leo_plugin_llm_deepseek::{DeepseekConfig, DeepseekLlmPlugin, DeepseekLlmProvider};
use serde_json::{Value, json};

#[derive(Clone, Debug)]
struct SeenRequest {
    path: String,
    authorization: Option<String>,
    x_api_key: Option<String>,
    anthropic_version: Option<String>,
    body: Value,
}

type Seen = Arc<Mutex<Vec<SeenRequest>>>;

fn record(seen: &Seen, uri: &Uri, headers: &HeaderMap, body: &Value) {
    let header = |name: &str| {
        headers
            .get(name)
            .and_then(|value| value.to_str().ok())
            .map(str::to_string)
    };
    seen.lock().unwrap().push(SeenRequest {
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
    record(&seen, &uri, &headers, &body);
    Json(json!({
        "id": "chatcmpl-test",
        "object": "chat.completion",
        "model": "deepseek-chat",
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
    record(&seen, &uri, &headers, &body);
    Json(json!({
        "id": "resp-test",
        "object": "response",
        "status": "completed",
        "model": "deepseek-chat",
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
    record(&seen, &uri, &headers, &body);
    Json(json!({
        "id": "msg-test",
        "type": "message",
        "role": "assistant",
        "model": "deepseek-chat",
        "content": [{"type": "text", "text": "Anthropic 回答"}],
        "stop_reason": "end_turn",
        "usage": {"input_tokens": 5, "output_tokens": 4}
    }))
}

async fn mock() -> (String, Seen) {
    let seen: Seen = Arc::default();
    let app = Router::new()
        .route("/chat/completions", post(chat_completions))
        .route("/responses", post(responses))
        .route("/v1/messages", post(messages))
        .with_state(seen.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    (format!("http://{addr}"), seen)
}

fn request(model: Option<&str>) -> ChatRequest {
    ChatRequest {
        model: model.map(str::to_string),
        messages: vec![
            ChatMessage::system("你是校对助手"),
            ChatMessage::user("第一段"),
            ChatMessage::user("第二段"),
        ],
        temperature: Some(0.3),
        top_p: None,
        max_tokens: Some(2048),
        stop: vec!["END".into()],
        ..Default::default()
    }
}

#[tokio::test]
async fn openai_chat_sends_documented_body_and_decodes_response() {
    let (base_url, seen) = mock().await;
    let provider = DeepseekLlmProvider::new(
        DeepseekConfig {
            base_url,
            ..Default::default()
        },
        "test-key".into(),
    )
    .unwrap();

    let response = provider
        .chat(&request(Some("deepseek-chat")))
        .await
        .unwrap();
    assert_eq!(response.content, "聊天回答");
    assert_eq!(response.usage.unwrap().total_tokens, 5);

    let seen = seen.lock().unwrap();
    assert_eq!(seen.len(), 1);
    let request = &seen[0];
    assert_eq!(request.path, "/chat/completions");
    assert_eq!(request.authorization.as_deref(), Some("Bearer test-key"));
    assert_eq!(request.body["model"], json!("deepseek-chat"));
    assert_eq!(request.body["messages"][0]["role"], json!("system"));
    assert_eq!(request.body["messages"][1]["content"], json!("第一段"));
    assert_eq!(request.body["max_tokens"], json!(2048));
}

#[tokio::test]
async fn anthropic_protocol_uses_x_api_key_and_system_field() {
    let (base_url, seen) = mock().await;
    let config = DeepseekConfig {
        protocol: leo_core::spi::LlmProtocol::Anthropic,
        base_url,
        ..Default::default()
    };
    let provider = DeepseekLlmProvider::new(config, "test-key".into()).unwrap();

    let response = provider
        .chat(&request(Some("deepseek-chat")))
        .await
        .unwrap();
    assert_eq!(response.content, "Anthropic 回答");

    let seen = seen.lock().unwrap();
    let request = &seen[0];
    assert_eq!(request.path, "/v1/messages");
    assert_eq!(request.x_api_key.as_deref(), Some("test-key"));
    assert_eq!(request.anthropic_version.as_deref(), Some("2023-06-01"));
    assert_eq!(request.body["system"], json!("你是校对助手"));
    assert_eq!(request.body["messages"].as_array().unwrap().len(), 1);
    assert_eq!(
        request.body["messages"][0]["content"],
        json!("第一段\n\n第二段")
    );
    assert_eq!(request.body["max_tokens"], json!(2048));
    assert_eq!(request.body["stop_sequences"], json!(["END"]));
}

#[tokio::test]
async fn responses_protocol_maps_instructions_and_input() {
    let (base_url, seen) = mock().await;
    let config = DeepseekConfig {
        protocol: leo_core::spi::LlmProtocol::OpenAiResponses,
        base_url,
        ..Default::default()
    };
    let provider = DeepseekLlmProvider::new(config, "test-key".into()).unwrap();

    let response = provider
        .chat(&request(Some("deepseek-chat")))
        .await
        .unwrap();
    assert_eq!(response.content, "响应回答");

    let seen = seen.lock().unwrap();
    let request = &seen[0];
    assert_eq!(request.path, "/responses");
    assert_eq!(request.body["instructions"], json!("你是校对助手"));
    assert_eq!(request.body["input"][0]["role"], json!("user"));
    assert_eq!(request.body["max_output_tokens"], json!(2048));
}

#[tokio::test]
async fn extra_body_is_merged_into_request() {
    let (base_url, seen) = mock().await;
    let mut extra_body = std::collections::BTreeMap::new();
    extra_body.insert("thinking".to_string(), json!({"type": "enabled"}));
    let config = DeepseekConfig {
        base_url,
        extra_body,
        ..Default::default()
    };
    let provider = DeepseekLlmProvider::new(config, "test-key".into()).unwrap();
    provider
        .chat(&request(Some("deepseek-chat")))
        .await
        .unwrap();

    let seen = seen.lock().unwrap();
    assert_eq!(seen[0].body["thinking"]["type"], json!("enabled"));
}

#[tokio::test]
async fn without_api_key_plugin_loads_but_registers_nothing() {
    let config = DeepseekConfig {
        api_key_env: "LEO_TEST_DEEPSEEK_KEY_NOT_SET".into(),
        ..Default::default()
    };
    let ctx = Context::new("deepseek_no_key");
    DeepseekLlmPlugin::new(config).apply(&ctx).await.unwrap();
    assert!(ctx.registry::<dyn LlmProvider>().get("deepseek").is_none());
}

#[tokio::test]
#[ignore = "需要 DEEPSEEK_API_KEY，会真实调用 DeepSeek 接口并计费"]
async fn live_chat() {
    let ctx = Context::new("deepseek_live");
    DeepseekLlmPlugin::default().apply(&ctx).await.unwrap();
    let provider = ctx
        .registry::<dyn LlmProvider>()
        .get("deepseek")
        .expect("没有 DEEPSEEK_API_KEY");
    let response = provider
        .chat(&ChatRequest {
            model: Some("deepseek-chat".into()),
            messages: vec![ChatMessage::user("用一句话介绍有声书制作")],
            max_tokens: Some(64),
            ..Default::default()
        })
        .await
        .unwrap();
    assert!(!response.content.is_empty());
}
