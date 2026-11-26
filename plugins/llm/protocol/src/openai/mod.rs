//! OpenAI 协议族（Chat Completions 与 Responses）的公共工具。

pub mod chat_completions;
pub mod responses;

use leo_core::error::{CoreError, CoreResult};
use leo_core::spi::{ChatMessage, ChatRole, FinishReason, ResponseFormat, TokenUsage};
use serde_json::Value;

/// 从字符串 / 内容块数组里抽取纯文本。
///
/// 覆盖常见形态：
/// - `"hello"` → `hello`
/// - `[{"type":"text","text":"hello"}]` → `hello`
/// - `[{"type":"input_text","text":"hello"}, ...]` → 多段用换行拼接
pub(crate) fn text_from_content(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Array(parts) => {
            let mut texts = Vec::new();
            for part in parts {
                match part {
                    Value::String(text) => texts.push(text.clone()),
                    Value::Object(map) => {
                        if let Some(text) = map.get("text").and_then(Value::as_str) {
                            texts.push(text.to_string());
                        } else if let Some(text) = map.get("content").and_then(Value::as_str) {
                            texts.push(text.to_string());
                        }
                    }
                    _ => {}
                }
            }
            texts.join("\n")
        }
        Value::Object(map) => map
            .get("text")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        _ => String::new(),
    }
}

/// OpenAI 角色字符串 → canonical 角色
pub(crate) fn parse_role(value: Option<&str>) -> CoreResult<ChatRole> {
    match value.unwrap_or_default() {
        "system" | "developer" => Ok(ChatRole::System),
        "user" => Ok(ChatRole::User),
        "assistant" => Ok(ChatRole::Assistant),
        other => Err(CoreError::InvalidParameter(format!(
            "不支持的 OpenAI 消息角色：{other:?}（只支持 system / developer / user / assistant）"
        ))),
    }
}

pub(crate) fn role_str(role: ChatRole) -> &'static str {
    match role {
        ChatRole::System => "system",
        ChatRole::User => "user",
        ChatRole::Assistant => "assistant",
    }
}

/// 把 system / developer 消息抽出来拼成一段，其余消息保持原顺序。
pub(crate) fn split_system(messages: &[ChatMessage]) -> (String, Vec<&ChatMessage>) {
    let mut system = Vec::new();
    let mut rest = Vec::new();
    for message in messages {
        if message.role == ChatRole::System {
            system.push(message.content.clone());
        } else {
            rest.push(message);
        }
    }
    (system.join("\n\n"), rest)
}

/// 上游结束原因 → canonical 结束原因
pub(crate) fn map_finish_reason(reason: Option<&str>) -> FinishReason {
    match reason.unwrap_or_default() {
        "stop" | "end_turn" | "stop_sequence" => FinishReason::Stop,
        "length" | "max_tokens" | "max_output_tokens" => FinishReason::Length,
        "content_filter" | "refusal" => FinishReason::ContentFilter,
        "tool_calls" | "tool_use" | "function_call" => FinishReason::ToolCalls,
        _ => FinishReason::Other,
    }
}

/// canonical 结束原因 → OpenAI 字符串
pub(crate) fn finish_reason_str(reason: FinishReason) -> &'static str {
    match reason {
        FinishReason::Stop | FinishReason::Other => "stop",
        FinishReason::Length => "length",
        FinishReason::ContentFilter => "content_filter",
        FinishReason::ToolCalls => "tool_calls",
    }
}

/// OpenAI usage（prompt/completion 或 input/output）→ canonical usage
pub(crate) fn usage_from_openai(usage: Option<&Value>) -> Option<TokenUsage> {
    let usage = usage?;
    let input = usage
        .get("prompt_tokens")
        .or_else(|| usage.get("input_tokens"))
        .and_then(Value::as_u64)
        .unwrap_or(0) as u32;
    let output = usage
        .get("completion_tokens")
        .or_else(|| usage.get("output_tokens"))
        .and_then(Value::as_u64)
        .unwrap_or(0) as u32;
    let total = usage
        .get("total_tokens")
        .and_then(Value::as_u64)
        .map(|value| value as u32)
        .unwrap_or(input.saturating_add(output));
    Some(TokenUsage {
        input_tokens: input,
        output_tokens: output,
        total_tokens: total,
    })
}

/// canonical usage → OpenAI Chat usage
pub(crate) fn usage_to_openai(usage: TokenUsage) -> Value {
    serde_json::json!({
        "prompt_tokens": usage.input_tokens,
        "completion_tokens": usage.output_tokens,
        "total_tokens": usage.total_tokens,
    })
}

/// OpenAI 的 `response_format` → canonical
pub(crate) fn parse_response_format(value: Option<&Value>) -> ResponseFormat {
    let Some(value) = value else {
        return ResponseFormat::Text;
    };
    match value.get("type").and_then(Value::as_str) {
        Some("json_object") => ResponseFormat::JsonObject,
        Some("json_schema") => {
            let schema = value
                .get("json_schema")
                .and_then(|json_schema| json_schema.get("schema"))
                .or_else(|| value.get("json_schema"))
                .cloned()
                .unwrap_or(Value::Null);
            ResponseFormat::JsonSchema { schema }
        }
        _ => ResponseFormat::Text,
    }
}

/// canonical `response_format` → OpenAI Chat 的 `response_format`
pub(crate) fn chat_response_format(format: &ResponseFormat) -> Option<Value> {
    match format {
        ResponseFormat::Text => None,
        ResponseFormat::JsonObject => Some(serde_json::json!({"type": "json_object"})),
        ResponseFormat::JsonSchema { schema } => Some(serde_json::json!({
            "type": "json_schema",
            "json_schema": {
                "name": "leo_response",
                "schema": schema,
                "strict": true,
            }
        })),
    }
}
