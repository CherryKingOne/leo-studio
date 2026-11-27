//! OpenAI Chat Completions（旧接口）编解码。
//!
//! 接口：`POST {base_url}/chat/completions`
//! 文档：<https://platform.openai.com/docs/api-reference/chat>
//!
//! 兼容点：`content` 可能是字符串，也可能是内容块数组；DeepSeek 等供应商还会在
//! `message.reasoning_content` 里返回推理过程，解码时统一放到 canonical 的
//! `reasoning_content`。

use leo_core::error::{CoreError, CoreResult};
use leo_core::spi::{ChatMessage, ChatRequest, ChatResponse};
use serde_json::{Map, Value, json};

use super::{
    chat_response_format, finish_reason_str, map_finish_reason, parse_response_format, parse_role,
    role_str, text_from_content, usage_from_openai, usage_to_openai,
};

/// canonical → `/chat/completions` 请求体
pub fn encode_request(request: &ChatRequest, model: &str) -> Value {
    let mut body = Map::new();
    body.insert("model".into(), json!(model));
    body.insert("messages".into(), json!(encode_messages(&request.messages)));
    if let Some(temperature) = request.temperature {
        body.insert("temperature".into(), json!(temperature));
    }
    if let Some(top_p) = request.top_p {
        body.insert("top_p".into(), json!(top_p));
    }
    if let Some(max_tokens) = request.max_tokens {
        body.insert("max_tokens".into(), json!(max_tokens));
    }
    if !request.stop.is_empty() {
        if request.stop.len() == 1 {
            body.insert("stop".into(), json!(request.stop[0]));
        } else {
            body.insert("stop".into(), json!(request.stop));
        }
    }
    if let Some(format) = chat_response_format(&request.response_format) {
        body.insert("response_format".into(), format);
    }
    crate::thinking::apply_chat(
        &mut body,
        &request.thinking,
        request.stream.unwrap_or(false),
    );
    Value::Object(body)
}

fn encode_messages(messages: &[ChatMessage]) -> Vec<Value> {
    messages
        .iter()
        .map(|message| {
            let mut value = Map::new();
            value.insert("role".into(), json!(role_str(message.role)));
            value.insert("content".into(), json!(message.content));
            if let Some(name) = &message.name {
                value.insert("name".into(), json!(name));
            }
            Value::Object(value)
        })
        .collect()
}

/// `/chat/completions` 请求体 → canonical
pub fn decode_request(body: &Value) -> CoreResult<ChatRequest> {
    let messages = body
        .get("messages")
        .and_then(Value::as_array)
        .ok_or_else(|| CoreError::InvalidParameter("OpenAI Chat 请求缺少 messages 数组".into()))?;
    let messages = messages
        .iter()
        .map(|message| {
            let role = parse_role(message.get("role").and_then(Value::as_str))?;
            Ok(ChatMessage {
                role,
                content: text_from_content(message.get("content").unwrap_or(&Value::Null)),
                name: message
                    .get("name")
                    .and_then(Value::as_str)
                    .map(str::to_string),
            })
        })
        .collect::<CoreResult<Vec<_>>>()?;

    Ok(ChatRequest {
        model: body
            .get("model")
            .and_then(Value::as_str)
            .map(str::to_string),
        messages,
        temperature: body
            .get("temperature")
            .and_then(Value::as_f64)
            .map(|v| v as f32),
        top_p: body.get("top_p").and_then(Value::as_f64).map(|v| v as f32),
        max_tokens: body
            .get("max_tokens")
            .and_then(Value::as_u64)
            .map(|v| v as u32),
        stop: parse_stop(body.get("stop")),
        response_format: parse_response_format(body.get("response_format")),
        stream: body.get("stream").and_then(Value::as_bool),
        thinking: crate::thinking::decode_chat(body),
    })
}

fn parse_stop(value: Option<&Value>) -> Vec<String> {
    match value {
        Some(Value::String(stop)) if !stop.is_empty() => vec![stop.clone()],
        Some(Value::Array(values)) => values
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_string)
            .collect(),
        _ => Vec::new(),
    }
}

/// `/chat/completions` 响应体 → canonical
pub fn decode_response(body: &Value) -> CoreResult<ChatResponse> {
    let choice = body
        .get("choices")
        .and_then(Value::as_array)
        .and_then(|choices| choices.first())
        .ok_or_else(|| CoreError::InvalidParameter("OpenAI Chat 响应缺少 choices[0]".into()))?;

    let message = choice.get("message").unwrap_or(&Value::Null);
    let content = text_from_content(message.get("content").unwrap_or(&Value::Null));
    let reasoning_content = message
        .get("reasoning_content")
        .or_else(|| message.get("reasoning"))
        .and_then(Value::as_str)
        .filter(|text| !text.is_empty())
        .map(str::to_string);

    Ok(ChatResponse {
        id: body.get("id").and_then(Value::as_str).map(str::to_string),
        model: body
            .get("model")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        content,
        reasoning_content,
        finish_reason: map_finish_reason(choice.get("finish_reason").and_then(Value::as_str)),
        usage: usage_from_openai(body.get("usage")),
    })
}

/// canonical → `/chat/completions` 响应体
pub fn encode_response(response: &ChatResponse) -> Value {
    let mut message = Map::new();
    message.insert("role".into(), json!("assistant"));
    message.insert("content".into(), json!(response.content));
    if let Some(reasoning) = &response.reasoning_content {
        message.insert("reasoning_content".into(), json!(reasoning));
    }

    let mut choice = Map::new();
    choice.insert("index".into(), json!(0));
    choice.insert("message".into(), Value::Object(message));
    choice.insert(
        "finish_reason".into(),
        json!(finish_reason_str(response.finish_reason)),
    );

    let mut body = Map::new();
    body.insert(
        "id".into(),
        json!(response.id.clone().unwrap_or_else(|| "chatcmpl-leo".into())),
    );
    body.insert("object".into(), json!("chat.completion"));
    body.insert("created".into(), json!(0));
    body.insert("model".into(), json!(response.model));
    body.insert("choices".into(), json!([Value::Object(choice)]));
    if let Some(usage) = response.usage {
        body.insert("usage".into(), usage_to_openai(usage));
    }
    Value::Object(body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use leo_core::spi::{ChatRole, FinishReason, ResponseFormat, TokenUsage};

    fn sample_response() -> ChatResponse {
        ChatResponse {
            id: Some("chatcmpl-test".into()),
            model: "deepseek-chat".into(),
            content: "你好".into(),
            reasoning_content: None,
            finish_reason: FinishReason::Stop,
            usage: Some(TokenUsage {
                input_tokens: 3,
                output_tokens: 2,
                total_tokens: 5,
            }),
        }
    }

    #[test]
    fn encode_request_matches_chat_shape() {
        let request = ChatRequest {
            model: Some("deepseek-chat".into()),
            messages: vec![
                ChatMessage::system("你是校对助手"),
                ChatMessage::user("改错别字"),
            ],
            temperature: Some(0.3),
            top_p: None,
            max_tokens: Some(2048),
            stop: vec!["\n\n".into()],
            response_format: ResponseFormat::JsonObject,
            ..Default::default()
        };
        let body = encode_request(&request, "deepseek-chat");
        assert_eq!(body["model"], json!("deepseek-chat"));
        assert_eq!(body["messages"][0]["role"], json!("system"));
        assert_eq!(body["messages"][1]["content"], json!("改错别字"));
        assert!((body["temperature"].as_f64().unwrap() - 0.3).abs() < 1e-6);
        assert_eq!(body["max_tokens"], json!(2048));
        assert_eq!(body["stop"], json!("\n\n"));
        assert_eq!(body["response_format"]["type"], json!("json_object"));
    }

    #[test]
    fn decode_request_accepts_content_parts_and_developer_role() {
        let body = json!({
            "model": "gpt-4.1",
            "messages": [
                {"role": "developer", "content": "遵守格式"},
                {"role": "user", "content": [{"type": "text", "text": "第一段"}, {"type": "text", "text": "第二段"}]}
            ],
            "stop": ["A", "B"],
            "response_format": {"type": "json_schema", "json_schema": {"name": "x", "schema": {"type": "object"}}}
        });
        let request = decode_request(&body).unwrap();
        assert_eq!(request.messages[0].role, ChatRole::System);
        assert_eq!(request.messages[0].content, "遵守格式");
        assert_eq!(request.messages[1].content, "第一段\n第二段");
        assert_eq!(request.stop, vec!["A".to_string(), "B".to_string()]);
        match request.response_format {
            ResponseFormat::JsonSchema { schema } => assert_eq!(schema["type"], json!("object")),
            other => panic!("unexpected format: {other:?}"),
        }
    }

    #[test]
    fn encode_request_maps_stream_and_reasoning_effort() {
        let request = ChatRequest {
            messages: vec![ChatMessage::user("你好")],
            stream: Some(true),
            thinking: leo_core::spi::ThinkingConfig {
                mode: leo_core::spi::ThinkingMode::Enabled,
                effort: Some(leo_core::spi::ReasoningEffort::High),
                budget_tokens: None,
            },
            ..Default::default()
        };
        let body = encode_request(&request, "deepseek-chat");
        assert_eq!(body["stream"], json!(true));
        assert_eq!(body["stream_options"]["include_usage"], json!(true));
        assert_eq!(body["reasoning_effort"], json!("high"));
    }

    #[test]
    fn decode_response_reads_reasoning_and_usage() {
        let body = json!({
            "id": "chatcmpl-1",
            "model": "deepseek-reasoner",
            "choices": [{
                "index": 0,
                "message": {"role": "assistant", "content": "答案", "reasoning_content": "思考过程"},
                "finish_reason": "stop"
            }],
            "usage": {"prompt_tokens": 10, "completion_tokens": 20, "total_tokens": 30}
        });
        let response = decode_response(&body).unwrap();
        assert_eq!(response.content, "答案");
        assert_eq!(response.reasoning_content.as_deref(), Some("思考过程"));
        assert_eq!(response.finish_reason, FinishReason::Stop);
        let usage = response.usage.unwrap();
        assert_eq!(
            (usage.input_tokens, usage.output_tokens, usage.total_tokens),
            (10, 20, 30)
        );
    }

    #[test]
    fn response_roundtrip() {
        let response = sample_response();
        let body = encode_response(&response);
        let back = decode_response(&body).unwrap();
        assert_eq!(back.content, response.content);
        assert_eq!(back.finish_reason, response.finish_reason);
        assert_eq!(back.usage, response.usage);
    }
}
