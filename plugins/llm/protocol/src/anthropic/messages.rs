//! Anthropic Messages（`POST /v1/messages`）编解码。
//!
//! 文档：<https://docs.anthropic.com/en/api/messages>
//!
//! 关键差异：
//! - `system` 是顶层字段，不在 messages 里
//! - `messages` 只允许 user / assistant，连续同角色要合并
//! - `max_tokens` 必填
//! - 没有原生 JSON 输出模式，canonical 的 JSON 要求会降级成 system 指令（有损）
//! - 响应的文本在 `content[]` 的 text / thinking 块里

use leo_core::error::{CoreError, CoreResult};
use leo_core::spi::{
    ChatMessage, ChatRequest, ChatResponse, ChatRole, FinishReason, ResponseFormat,
};
use serde_json::{Map, Value, json};

use crate::openai::{
    map_finish_reason, role_str, split_system, text_from_content, usage_from_openai,
};

/// Anthropic 要求 `max_tokens` 必填；canonical 没给时用这个兜底。
pub const DEFAULT_MAX_TOKENS: u32 = 4096;

/// canonical → `/v1/messages` 请求体
pub fn encode_request(request: &ChatRequest, model: &str) -> Value {
    let (mut system, messages) = split_system(&request.messages);
    if let Some(instruction) = json_instruction(&request.response_format) {
        if system.is_empty() {
            system = instruction;
        } else {
            system = format!("{system}\n\n{instruction}");
        }
    }

    let mut body = Map::new();
    body.insert("model".into(), json!(model));
    body.insert(
        "max_tokens".into(),
        json!(request.max_tokens.unwrap_or(DEFAULT_MAX_TOKENS)),
    );
    if !system.is_empty() {
        body.insert("system".into(), json!(system));
    }
    body.insert("messages".into(), json!(encode_messages(&messages)));
    if let Some(temperature) = request.temperature {
        body.insert("temperature".into(), json!(temperature));
    }
    if let Some(top_p) = request.top_p {
        body.insert("top_p".into(), json!(top_p));
    }
    if !request.stop.is_empty() {
        body.insert("stop_sequences".into(), json!(request.stop));
    }
    Value::Object(body)
}

/// Anthropic 要求消息交替出现；连续的 user / assistant 合并成一条，用空行隔开。
fn encode_messages(messages: &[&ChatMessage]) -> Vec<Value> {
    let mut merged: Vec<(ChatRole, String)> = Vec::new();
    for message in messages {
        if let Some(last) = merged.last_mut()
            && last.0 == message.role
        {
            last.1.push_str("\n\n");
            last.1.push_str(&message.content);
            continue;
        }
        merged.push((message.role, message.content.clone()));
    }
    merged
        .into_iter()
        .map(|(role, content)| {
            json!({
                "role": role_str(role),
                "content": content,
            })
        })
        .collect()
}

/// Anthropic 没有 response_format；用 system 指令近似。
fn json_instruction(format: &ResponseFormat) -> Option<String> {
    match format {
        ResponseFormat::Text => None,
        ResponseFormat::JsonObject => {
            Some("请只输出一个合法的 JSON 对象，不要输出任何解释、Markdown 代码块或前后缀。".into())
        }
        ResponseFormat::JsonSchema { schema } => {
            let schema = serde_json::to_string_pretty(schema).unwrap_or_else(|_| "{}".into());
            Some(format!(
                "请只输出符合以下 JSON Schema 的 JSON，不要输出任何解释、Markdown 代码块或前后缀。\n{schema}"
            ))
        }
    }
}

/// `/v1/messages` 请求体 → canonical
pub fn decode_request(body: &Value) -> CoreResult<ChatRequest> {
    let mut messages = Vec::new();
    if let Some(system) = body.get("system") {
        let text = text_from_content(system);
        if !text.is_empty() {
            messages.push(ChatMessage::system(text));
        }
    }
    let raw_messages = body
        .get("messages")
        .and_then(Value::as_array)
        .ok_or_else(|| CoreError::InvalidParameter("Anthropic 请求缺少 messages 数组".into()))?;
    for message in raw_messages {
        let role = match message
            .get("role")
            .and_then(Value::as_str)
            .unwrap_or_default()
        {
            "user" => ChatRole::User,
            "assistant" => ChatRole::Assistant,
            other => {
                return Err(CoreError::InvalidParameter(format!(
                    "Anthropic 消息角色只支持 user / assistant，收到 {other:?}"
                )));
            }
        };
        messages.push(ChatMessage {
            role,
            content: text_from_content(message.get("content").unwrap_or(&Value::Null)),
            name: None,
        });
    }

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
        stop: body
            .get("stop_sequences")
            .and_then(Value::as_array)
            .map(|values| {
                values
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default(),
        // Anthropic 没有原生 response_format，解码时按普通文本处理。
        response_format: ResponseFormat::Text,
    })
}

/// `/v1/messages` 响应体 → canonical
pub fn decode_response(body: &Value) -> CoreResult<ChatResponse> {
    let content_blocks = body
        .get("content")
        .and_then(Value::as_array)
        .ok_or_else(|| CoreError::InvalidParameter("Anthropic 响应缺少 content 数组".into()))?;

    let mut content = String::new();
    let mut reasoning = String::new();
    for block in content_blocks {
        match block.get("type").and_then(Value::as_str) {
            Some("text") => {
                if let Some(text) = block.get("text").and_then(Value::as_str) {
                    content.push_str(text);
                }
            }
            Some("thinking") => {
                if let Some(text) = block.get("thinking").and_then(Value::as_str) {
                    reasoning.push_str(text);
                }
            }
            _ => {}
        }
    }

    Ok(ChatResponse {
        id: body.get("id").and_then(Value::as_str).map(str::to_string),
        model: body
            .get("model")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        content,
        reasoning_content: (!reasoning.is_empty()).then_some(reasoning),
        finish_reason: map_finish_reason(body.get("stop_reason").and_then(Value::as_str)),
        usage: usage_from_openai(body.get("usage")),
    })
}

/// canonical → `/v1/messages` 响应体
pub fn encode_response(response: &ChatResponse) -> Value {
    let mut content = Vec::new();
    if let Some(reasoning) = &response.reasoning_content {
        content.push(json!({"type": "thinking", "thinking": reasoning}));
    }
    content.push(json!({"type": "text", "text": response.content}));

    let stop_reason = match response.finish_reason {
        FinishReason::Stop | FinishReason::Other => "end_turn",
        FinishReason::Length => "max_tokens",
        FinishReason::ContentFilter => "refusal",
        FinishReason::ToolCalls => "tool_use",
    };

    let mut body = Map::new();
    body.insert(
        "id".into(),
        json!(response.id.clone().unwrap_or_else(|| "msg_leo".into())),
    );
    body.insert("type".into(), json!("message"));
    body.insert("role".into(), json!("assistant"));
    body.insert("model".into(), json!(response.model));
    body.insert("content".into(), json!(content));
    body.insert("stop_reason".into(), json!(stop_reason));
    body.insert("stop_sequence".into(), Value::Null);
    if let Some(usage) = response.usage {
        body.insert(
            "usage".into(),
            json!({
                "input_tokens": usage.input_tokens,
                "output_tokens": usage.output_tokens,
            }),
        );
    }
    Value::Object(body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use leo_core::spi::TokenUsage;

    #[test]
    fn encode_request_maps_system_and_merges_roles() {
        let request = ChatRequest {
            model: Some("claude-sonnet".into()),
            messages: vec![
                ChatMessage::system("你是校对助手"),
                ChatMessage::user("第一段"),
                ChatMessage::user("第二段"),
                ChatMessage::assistant("好的"),
            ],
            temperature: Some(0.4),
            top_p: None,
            max_tokens: Some(2048),
            stop: vec!["END".into()],
            response_format: ResponseFormat::JsonObject,
        };
        let body = encode_request(&request, "claude-sonnet");
        assert_eq!(body["model"], json!("claude-sonnet"));
        assert_eq!(body["max_tokens"], json!(2048));
        assert!(body["system"].as_str().unwrap().contains("JSON"));
        assert!(body["system"].as_str().unwrap().contains("你是校对助手"));
        assert_eq!(body["messages"].as_array().unwrap().len(), 2);
        assert_eq!(body["messages"][0]["role"], json!("user"));
        assert_eq!(body["messages"][0]["content"], json!("第一段\n\n第二段"));
        assert_eq!(body["messages"][1]["role"], json!("assistant"));
        assert_eq!(body["stop_sequences"], json!(["END"]));
    }

    #[test]
    fn decode_request_reads_system_and_content_blocks() {
        let body = json!({
            "model": "claude-sonnet",
            "system": "遵守格式",
            "messages": [
                {"role": "user", "content": [{"type": "text", "text": "你好"}]},
                {"role": "assistant", "content": "在"}
            ],
            "max_tokens": 512,
            "temperature": 0.2,
            "stop_sequences": ["END"]
        });
        let request = decode_request(&body).unwrap();
        assert_eq!(request.messages.len(), 3);
        assert_eq!(request.messages[0].content, "遵守格式");
        assert_eq!(request.messages[1].content, "你好");
        assert_eq!(request.max_tokens, Some(512));
        assert_eq!(request.stop, vec!["END".to_string()]);
    }

    #[test]
    fn decode_request_reads_system_blocks() {
        let body = json!({
            "model": "claude-sonnet",
            "system": [
                {"type": "text", "text": "遵守格式"},
                {"type": "text", "text": "简洁回答"}
            ],
            "messages": [{"role": "user", "content": "你好"}],
            "max_tokens": 512
        });
        let request = decode_request(&body).unwrap();
        assert_eq!(request.messages[0].content, "遵守格式\n简洁回答");
    }

    #[test]
    fn decode_response_reads_text_and_thinking() {
        let body = json!({
            "id": "msg_1",
            "type": "message",
            "role": "assistant",
            "model": "claude-sonnet",
            "content": [
                {"type": "thinking", "thinking": "先想一下"},
                {"type": "text", "text": "最终答案"}
            ],
            "stop_reason": "end_turn",
            "usage": {"input_tokens": 4, "output_tokens": 6}
        });
        let response = decode_response(&body).unwrap();
        assert_eq!(response.content, "最终答案");
        assert_eq!(response.reasoning_content.as_deref(), Some("先想一下"));
        assert_eq!(response.finish_reason, FinishReason::Stop);
        assert_eq!(
            response.usage,
            Some(TokenUsage {
                input_tokens: 4,
                output_tokens: 6,
                total_tokens: 10
            })
        );
    }

    #[test]
    fn response_roundtrip() {
        let response = ChatResponse {
            id: Some("msg_test".into()),
            model: "claude-sonnet".into(),
            content: "答案".into(),
            reasoning_content: Some("思考".into()),
            finish_reason: FinishReason::Stop,
            usage: Some(TokenUsage {
                input_tokens: 3,
                output_tokens: 5,
                total_tokens: 8,
            }),
        };
        let body = encode_response(&response);
        let back = decode_response(&body).unwrap();
        assert_eq!(back.content, response.content);
        assert_eq!(back.reasoning_content, response.reasoning_content);
        assert_eq!(back.usage, response.usage);
    }
}
