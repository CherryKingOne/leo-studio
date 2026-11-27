//! 用本地假服务器模拟火山方舟的三种协议入口（不需要 API Key、不联网）。
//! 真实调用测试：设置 ARK_API_KEY 和 ARK_MODEL（推理接入点 `ep-...`）后运行
//! `cargo test -p leo-plugin-llm-volcengine live_chat -- --ignored --nocapture`。

use std::sync::{Arc, Mutex};

use axum::Json;
use axum::Router;
use axum::extract::State;
use axum::http::{HeaderMap, Uri, header};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use futures_util::StreamExt;
use leo_core::context::Context;
use leo_core::plugin::Plugin;
use leo_core::spi::{ChatMessage, ChatRequest, LlmProtocol, LlmProvider};
use leo_plugin_llm_volcengine::{VolcengineConfig, VolcengineLlmPlugin, VolcengineLlmProvider};
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
        "model": "ep-2026xxxx",
        "choices": [{
            "index": 0,
            "message": {"role": "assistant", "content": "豆包回答"},
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
        "model": "ep-2026xxxx",
        "output": [{
            "type": "message",
            "role": "assistant",
            "content": [{"type": "output_text", "text": "豆包响应回答"}]
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
        "model": "ep-2026xxxx",
        "content": [{"type": "text", "text": "豆包 Anthropic 回答"}],
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

async fn mock_sse() -> String {
    async fn stream() -> Response {
        let body = concat!(
            "data: {\"id\":\"1\",\"object\":\"chat.completion.chunk\",\"choices\":[{\"index\":0,\"delta\":{\"role\":\"assistant\",\"content\":\"豆包\"},\"finish_reason\":null}]}\n\n",
            "data: {\"id\":\"1\",\"object\":\"chat.completion.chunk\",\"choices\":[{\"index\":0,\"delta\":{\"reasoning_content\":\"思考\"},\"finish_reason\":null}]}\n\n",
            "data: {\"id\":\"1\",\"object\":\"chat.completion.chunk\",\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":3,\"completion_tokens\":2,\"total_tokens\":5}}\n\n",
            "data: [DONE]\n\n",
        );
        ([(header::CONTENT_TYPE, "text/event-stream")], body).into_response()
    }

    let app = Router::new().route("/chat/completions", post(stream));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    format!("http://{addr}")
}

fn request() -> ChatRequest {
    ChatRequest {
        messages: vec![
            ChatMessage::system("你是有声书编辑"),
            ChatMessage::user("给这章写摘要"),
        ],
        temperature: Some(0.3),
        max_tokens: Some(2048),
        ..Default::default()
    }
}

#[tokio::test]
async fn openai_chat_uses_endpoint_model_and_thinking() {
    let (base_url, seen) = mock().await;
    let config = VolcengineConfig {
        base_url,
        model: "ep-2026xxxx".into(),
        thinking: Some("enabled".into()),
        stream: false,
        ..Default::default()
    };
    let provider = VolcengineLlmProvider::new(config, "test-key".into()).unwrap();

    let response = provider.chat(&request()).await.unwrap();
    assert_eq!(response.content, "豆包回答");

    let seen = seen.lock().unwrap();
    let request = &seen[0];
    assert_eq!(request.path, "/chat/completions");
    assert_eq!(request.authorization.as_deref(), Some("Bearer test-key"));
    assert_eq!(request.body["model"], json!("ep-2026xxxx"));
    assert_eq!(request.body["thinking"]["type"], json!("enabled"));
    assert_eq!(request.body["messages"][0]["role"], json!("system"));
}

#[tokio::test]
async fn anthropic_protocol_uses_x_api_key() {
    let (base_url, seen) = mock().await;
    let config = VolcengineConfig {
        protocol: LlmProtocol::Anthropic,
        base_url,
        model: "ep-2026xxxx".into(),
        stream: false,
        ..Default::default()
    };
    let provider = VolcengineLlmProvider::new(config, "test-key".into()).unwrap();

    let response = provider.chat(&request()).await.unwrap();
    assert_eq!(response.content, "豆包 Anthropic 回答");

    let seen = seen.lock().unwrap();
    let request = &seen[0];
    assert_eq!(request.path, "/v1/messages");
    assert_eq!(request.x_api_key.as_deref(), Some("test-key"));
    assert_eq!(request.anthropic_version.as_deref(), Some("2023-06-01"));
    assert_eq!(request.body["system"], json!("你是有声书编辑"));
}

#[tokio::test]
async fn responses_protocol_maps_instructions() {
    let (base_url, seen) = mock().await;
    let config = VolcengineConfig {
        protocol: LlmProtocol::OpenAiResponses,
        base_url,
        model: "ep-2026xxxx".into(),
        stream: false,
        ..Default::default()
    };
    let provider = VolcengineLlmProvider::new(config, "test-key".into()).unwrap();

    let response = provider.chat(&request()).await.unwrap();
    assert_eq!(response.content, "豆包响应回答");

    let seen = seen.lock().unwrap();
    assert_eq!(seen[0].path, "/responses");
    assert_eq!(seen[0].body["instructions"], json!("你是有声书编辑"));
    assert_eq!(seen[0].body["max_output_tokens"], json!(2048));
}

#[tokio::test]
async fn default_chat_uses_sse_and_aggregates() {
    let base_url = mock_sse().await;
    let provider = VolcengineLlmProvider::new(
        VolcengineConfig {
            base_url,
            model: "ep-2026xxxx".into(),
            stream: true,
            ..Default::default()
        },
        "test-key".into(),
    )
    .unwrap();

    let response = provider.chat(&request()).await.unwrap();
    assert_eq!(response.content, "豆包");
    assert_eq!(response.reasoning_content.as_deref(), Some("思考"));
    assert_eq!(response.usage.unwrap().total_tokens, 5);
}

#[tokio::test]
async fn chat_stream_yields_incremental_deltas() {
    let base_url = mock_sse().await;
    let provider = VolcengineLlmProvider::new(
        VolcengineConfig {
            base_url,
            model: "ep-2026xxxx".into(),
            stream: true,
            ..Default::default()
        },
        "test-key".into(),
    )
    .unwrap();

    let mut stream = provider.chat_stream(&request()).await.unwrap();
    let mut content = String::new();
    let mut reasoning = String::new();
    let mut deltas = 0;
    while let Some(item) = stream.next().await {
        let delta = item.unwrap();
        deltas += 1;
        if let Some(part) = delta.content {
            content.push_str(&part);
        }
        if let Some(part) = delta.reasoning_content {
            reasoning.push_str(&part);
        }
    }
    assert_eq!(content, "豆包");
    assert_eq!(reasoning, "思考");
    assert!(deltas >= 3);
}

#[tokio::test]
async fn without_api_key_plugin_loads_but_registers_nothing() {
    let config = VolcengineConfig {
        api_key_env: "LEO_TEST_ARK_KEY_NOT_SET".into(),
        ..Default::default()
    };
    let ctx = Context::new("volcengine_no_key");
    VolcengineLlmPlugin::new(config).apply(&ctx).await.unwrap();
    assert!(
        ctx.registry::<dyn LlmProvider>()
            .get("volcengine")
            .is_none()
    );
}

#[tokio::test]
#[ignore = "需要 ARK_API_KEY + ARK_MODEL（推理接入点 ep-...），会真实调用火山方舟接口并计费"]
async fn live_chat() {
    let model = std::env::var("ARK_MODEL")
        .expect("需要 ARK_MODEL，填火山方舟控制台的推理接入点 ID（ep-...）");
    let ctx = Context::new("volcengine_live");
    VolcengineLlmPlugin::new(VolcengineConfig {
        model,
        ..Default::default()
    })
    .apply(&ctx)
    .await
    .unwrap();
    let provider = ctx
        .registry::<dyn LlmProvider>()
        .get("volcengine")
        .expect("没有 ARK_API_KEY");
    let response = provider
        .chat(&ChatRequest {
            messages: vec![ChatMessage::user("用一句话介绍有声书制作")],
            max_tokens: Some(64),
            ..Default::default()
        })
        .await
        .unwrap();
    assert!(!response.content.is_empty());
}
