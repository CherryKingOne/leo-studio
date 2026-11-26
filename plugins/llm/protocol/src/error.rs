//! 三家上游错误体 → 统一的 [`CoreError`]。
//!
//! - OpenAI：`{"error":{"message","type","code"}}`
//! - Anthropic：`{"type":"error","error":{"type","message"}}`
//! - 部分兼容网关会直接返回 `{"message":"..."}` 或纯文本

use leo_core::error::CoreError;
use leo_core::spi::LlmProtocol;
use serde_json::Value;

pub fn decode_error(plugin_id: &str, protocol: LlmProtocol, status: u16, body: &str) -> CoreError {
    let detail = detail_from_body(body);
    CoreError::Plugin {
        plugin_id: plugin_id.into(),
        message: format!(
            "LLM 接口调用失败（{}，HTTP {status}）：{detail}",
            protocol.as_str()
        ),
    }
}

fn detail_from_body(body: &str) -> String {
    let Ok(value) = serde_json::from_str::<Value>(body) else {
        return truncate(body);
    };

    if let Some(error) = value.get("error") {
        if let Some(message) = error.get("message").and_then(Value::as_str) {
            let kind = error
                .get("type")
                .or_else(|| error.get("code"))
                .and_then(Value::as_str);
            return match kind {
                Some(kind) if !kind.is_empty() => format!("[{kind}] {message}"),
                _ => message.to_string(),
            };
        }
        if let Some(message) = error.as_str() {
            return message.to_string();
        }
    }
    if let Some(message) = value.get("message").and_then(Value::as_str) {
        return message.to_string();
    }
    truncate(body)
}

fn truncate(body: &str) -> String {
    body.chars().take(300).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn openai_error() {
        let body = r#"{"error":{"message":"model not found","type":"invalid_request_error","code":"model_not_found"}}"#;
        let err = decode_error(
            "leo-plugin-llm-deepseek",
            LlmProtocol::OpenAiChat,
            404,
            body,
        );
        let text = err.to_string();
        assert!(text.contains("model not found"), "{text}");
        assert!(text.contains("invalid_request_error"), "{text}");
        assert!(text.contains("openai-chat"), "{text}");
    }

    #[test]
    fn anthropic_error() {
        let body = r#"{"type":"error","error":{"type":"authentication_error","message":"invalid x-api-key"}}"#;
        let err = decode_error("leo-plugin-llm-bailian", LlmProtocol::Anthropic, 401, body);
        let text = err.to_string();
        assert!(text.contains("invalid x-api-key"), "{text}");
        assert!(text.contains("anthropic"), "{text}");
    }

    #[test]
    fn plain_text_error_is_truncated() {
        let body = "x".repeat(500);
        let detail = detail_from_body(&body);
        assert_eq!(detail.chars().count(), 300);
    }
}
