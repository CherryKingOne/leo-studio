//! OpenAI Responses API（新接口）编解码。
//!
//! 接口：`POST {base_url}/responses`
//! 文档：<https://platform.openai.com/docs/api-reference/responses>
//!
//! 与 Chat Completions 的主要差异：
//! - system 提示放在顶层 `instructions`，用户/助手消息放在 `input`
//! - 输出上限叫 `max_output_tokens`
//! - JSON 输出走 `text.format`
//! - 响应是 `output` 项目数组，文本在 `output_text` / `message` 项目里

use leo_core::error::CoreResult;
use leo_core::spi::{ChatMessage, ChatRequest, ChatResponse, FinishReason, ResponseFormat};
use serde_json::{Map, Value, json};

use super::{
    finish_reason_str, map_finish_reason, parse_role, text_from_content, usage_from_openai,
};

/// canonical → `/responses` 请求体
pub fn encode_request(request: &ChatRequest, model: &str) -> Value {
    let (instructions, messages) = super::split_system(&request.messages);
    let mut body = Map::new();
    body.insert("model".into(), json!(model));
    body.insert("input".into(), json!(encode_input(&messages)));
    if !instructions.is_empty() {
        body.insert("instructions".into(), json!(instructions));
    }
    if let Some(temperature) = request.temperature {
        body.insert("temperature".into(), json!(temperature));
    }
    if let Some(top_p) = request.top_p {
        body.insert("top_p".into(), json!(top_p));
    }
    if let Some(max_tokens) = request.max_tokens {
        body.insert("max_output_tokens".into(), json!(max_tokens));
    }
    if let Some(format) = responses_text_format(&request.response_format) {
        body.insert("text".into(), json!({ "format": format }));
    }
    crate::thinking::apply_responses(
        &mut body,
        &request.thinking,
        request.stream.unwrap_or(false),
    );
    // Responses API 没有原生 stop 参数；canonical.stop 在这里忽略（有损转换）
    Value::Object(body)
}

fn encode_input(messages: &[&ChatMessage]) -> Vec<Value> {
    messages
        .iter()
        .map(|message| {
            let content_type = if message.role == leo_core::spi::ChatRole::Assistant {
                "output_text"
            } else {
                "input_text"
            };
            json!({
                "type": "message",
                "role": super::role_str(message.role),
                "content": [{
                    "type": content_type,
                    "text": message.content,
                }],
            })
        })
        .collect()
}

fn responses_text_format(format: &ResponseFormat) -> Option<Value> {
    match format {
        ResponseFormat::Text => None,
        ResponseFormat::JsonObject => Some(json!({"type": "json_object"})),
        ResponseFormat::JsonSchema { schema } => Some(json!({
            "type": "json_schema",
            "name": "leo_response",
            "schema": schema,
            "strict": true,
        })),
    }
}

/// `/responses` 请求体 → canonical
pub fn decode_request(body: &Value) -> CoreResult<ChatRequest> {
    let mut messages = Vec::new();
    if let Some(instructions) = body.get("instructions").and_then(Value::as_str)
        && !instructions.is_empty()
    {
        messages.push(ChatMessage::system(instructions));
    }
    match body.get("input") {
        Some(Value::String(text)) => messages.push(ChatMessage::user(text.clone())),
        Some(Value::Array(items)) => {
            for item in items {
                if let Some(text) = item.as_str() {
                    messages.push(ChatMessage::user(text.to_string()));
                    continue;
                }
                let item_type = item
                    .get("type")
                    .and_then(Value::as_str)
                    .unwrap_or("message");
                if item_type != "message" {
                    continue;
                }
                let role = parse_role(item.get("role").and_then(Value::as_str))?;
                messages.push(ChatMessage {
                    role,
                    content: text_from_content(item.get("content").unwrap_or(&Value::Null)),
                    name: None,
                });
            }
        }
        _ => {}
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
            .get("max_output_tokens")
            .and_then(Value::as_u64)
            .map(|v| v as u32),
        stop: Vec::new(),
        response_format: parse_responses_format(body),
        stream: body.get("stream").and_then(Value::as_bool),
        thinking: crate::thinking::decode_responses(body),
    })
}

fn parse_responses_format(body: &Value) -> ResponseFormat {
    let Some(format) = body.get("text").and_then(|text| text.get("format")) else {
        return ResponseFormat::Text;
    };
    match format.get("type").and_then(Value::as_str) {
        Some("json_object") => ResponseFormat::JsonObject,
        Some("json_schema") => ResponseFormat::JsonSchema {
            schema: format.get("schema").cloned().unwrap_or(Value::Null),
        },
        _ => ResponseFormat::Text,
    }
}

/// `/responses` 响应体 → canonical
pub fn decode_response(body: &Value) -> CoreResult<ChatResponse> {
    let mut content = String::new();
    let mut reasoning = String::new();

    if let Some(output) = body.get("output").and_then(Value::as_array) {
        for item in output {
            match item.get("type").and_then(Value::as_str) {
                Some("message") => {
                    let role = item
                        .get("role")
                        .and_then(Value::as_str)
                        .unwrap_or("assistant");
                    if role == "assistant" {
                        content.push_str(&text_from_content(
                            item.get("content").unwrap_or(&Value::Null),
                        ));
                    }
                }
                Some("reasoning") => {
                    if let Some(summary) = item.get("summary").and_then(Value::as_array) {
                        for part in summary {
                            if let Some(text) = part.get("text").and_then(Value::as_str) {
                                reasoning.push_str(text);
                            }
                        }
                    }
                }
                Some("output_text") => {
                    if let Some(text) = item.get("text").and_then(Value::as_str) {
                        content.push_str(text);
                    }
                }
                _ => {}
            }
        }
    }

    if content.is_empty()
        && let Some(text) = body.get("output_text").and_then(Value::as_str)
    {
        content.push_str(text);
    }

    let status = body
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("completed");
    let incomplete_reason = body
        .get("incomplete_details")
        .and_then(|details| details.get("reason"))
        .and_then(Value::as_str);
    let finish_reason = match status {
        "completed" => FinishReason::Stop,
        "incomplete" => {
            if incomplete_reason.is_some_and(|reason| reason.contains("max_output_tokens")) {
                FinishReason::Length
            } else {
                FinishReason::Other
            }
        }
        _ => map_finish_reason(incomplete_reason),
    };

    Ok(ChatResponse {
        id: body.get("id").and_then(Value::as_str).map(str::to_string),
        model: body
            .get("model")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        content,
        reasoning_content: (!reasoning.is_empty()).then_some(reasoning),
        finish_reason,
        usage: usage_from_openai(body.get("usage")),
    })
}

/// canonical → `/responses` 响应体
pub fn encode_response(response: &ChatResponse) -> Value {
    let message = json!({
        "type": "message",
        "role": "assistant",
        "content": [{
            "type": "output_text",
            "text": response.content,
        }],
    });
    let mut output = vec![message];
    if let Some(reasoning) = &response.reasoning_content {
        output.push(json!({
            "type": "reasoning",
            "summary": [{
                "type": "summary_text",
                "text": reasoning,
            }],
        }));
    }

    let incomplete = response.finish_reason == FinishReason::Length;
    let mut body = Map::new();
    body.insert(
        "id".into(),
        json!(response.id.clone().unwrap_or_else(|| "resp_leo".into())),
    );
    body.insert("object".into(), json!("response"));
    body.insert("created_at".into(), json!(0));
    body.insert("model".into(), json!(response.model));
    body.insert(
        "status".into(),
        json!(if incomplete {
            "incomplete"
        } else {
            "completed"
        }),
    );
    if incomplete {
        body.insert(
            "incomplete_details".into(),
            json!({"reason": "max_output_tokens"}),
        );
    }
    body.insert("output".into(), json!(output));
    body.insert("output_text".into(), json!(response.content));
    body.insert(
        "finish_reason".into(),
        json!(finish_reason_str(response.finish_reason)),
    );
    if let Some(usage) = response.usage {
        body.insert(
            "usage".into(),
            json!({
                "input_tokens": usage.input_tokens,
                "output_tokens": usage.output_tokens,
                "total_tokens": usage.total_tokens,
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
    fn encode_request_maps_instructions_and_text_format() {
        let request = ChatRequest {
            model: Some("gpt-4.1".into()),
            messages: vec![
                ChatMessage::system("你是校对助手"),
                ChatMessage::user("改错别字"),
                ChatMessage::assistant("好的"),
            ],
            temperature: Some(0.2),
            max_tokens: Some(1024),
            response_format: ResponseFormat::JsonObject,
            ..Default::default()
        };
        let body = encode_request(&request, "gpt-4.1");
        assert_eq!(body["model"], json!("gpt-4.1"));
        assert_eq!(body["instructions"], json!("你是校对助手"));
        assert_eq!(body["input"][0]["content"][0]["type"], json!("input_text"));
        assert_eq!(body["input"][1]["content"][0]["type"], json!("output_text"));
        assert_eq!(body["max_output_tokens"], json!(1024));
        assert_eq!(body["text"]["format"]["type"], json!("json_object"));
    }

    #[test]
    fn decode_request_reads_input_array() {
        let body = json!({
            "model": "gpt-4.1",
            "instructions": "遵守格式",
            "input": [
                {"type": "message", "role": "user", "content": [{"type": "input_text", "text": "你好"}]},
                {"type": "message", "role": "assistant", "content": [{"type": "output_text", "text": "你好"}]}
            ],
            "max_output_tokens": 512,
            "text": {"format": {"type": "json_schema", "name": "x", "schema": {"type": "object"}}}
        });
        let request = decode_request(&body).unwrap();
        assert_eq!(request.messages.len(), 3);
        assert_eq!(request.messages[0].content, "遵守格式");
        assert_eq!(request.messages[1].content, "你好");
        assert_eq!(request.max_tokens, Some(512));
        match request.response_format {
            ResponseFormat::JsonSchema { schema } => assert_eq!(schema["type"], json!("object")),
            other => panic!("unexpected format: {other:?}"),
        }
    }

    #[test]
    fn encode_request_maps_stream_and_reasoning() {
        let request = ChatRequest {
            messages: vec![ChatMessage::user("你好")],
            stream: Some(true),
            thinking: leo_core::spi::ThinkingConfig {
                mode: leo_core::spi::ThinkingMode::Enabled,
                effort: Some(leo_core::spi::ReasoningEffort::Medium),
                budget_tokens: None,
            },
            ..Default::default()
        };
        let body = encode_request(&request, "gpt-5");
        assert_eq!(body["stream"], json!(true));
        assert_eq!(body["reasoning"]["effort"], json!("medium"));
    }

    #[test]
    fn decode_response_reads_output_items_and_usage() {
        let body = json!({
            "id": "resp_1",
            "model": "gpt-4.1",
            "status": "completed",
            "output": [
                {"type": "reasoning", "summary": [{"type": "summary_text", "text": "先想一下"}]},
                {"type": "message", "role": "assistant", "content": [{"type": "output_text", "text": "最终答案"}]}
            ],
            "usage": {"input_tokens": 5, "output_tokens": 7, "total_tokens": 12}
        });
        let response = decode_response(&body).unwrap();
        assert_eq!(response.content, "最终答案");
        assert_eq!(response.reasoning_content.as_deref(), Some("先想一下"));
        assert_eq!(response.finish_reason, FinishReason::Stop);
        assert_eq!(
            response.usage,
            Some(TokenUsage {
                input_tokens: 5,
                output_tokens: 7,
                total_tokens: 12
            })
        );
    }

    #[test]
    fn response_roundtrip() {
        let response = ChatResponse {
            id: Some("resp_test".into()),
            model: "gpt-4.1".into(),
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
