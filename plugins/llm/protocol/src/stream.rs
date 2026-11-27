//! 流式响应：SSE 解析 + 三套协议的增量解码。
//!
//! 各供应商共用 `SseParser` 切 `data:` 帧，再用 `decode_stream_event` 把帧转成
//! canonical `ChatDelta`。这样流式细节也留在协议库里，供应商只负责发 HTTP 请求。

use leo_core::error::{CoreError, CoreResult};
use leo_core::spi::{ChatDelta, FinishReason, LlmProtocol, TokenUsage};
use serde_json::Value;

/// 增量式 SSE 解析器。
///
/// 用法：每读到一段 bytes 就 `feed`，得到完整的 `data:` 载荷；流结束时调用 `finish`。
#[derive(Debug, Default)]
pub struct SseParser {
    buffer: Vec<u8>,
    data_lines: Vec<String>,
}

impl SseParser {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn feed(&mut self, bytes: &[u8]) -> Vec<String> {
        self.buffer.extend_from_slice(bytes);
        let mut payloads = Vec::new();
        while let Some(newline) = self.buffer.iter().position(|byte| *byte == b'\n') {
            let line = self.buffer[..newline].to_vec();
            self.buffer.drain(..=newline);
            let line = String::from_utf8_lossy(&line);
            self.handle_line(line.trim_end_matches('\r'), &mut payloads);
        }
        payloads
    }

    /// 处理流结束时残留的最后一行（有些服务端不会补空行）。
    pub fn finish(&mut self) -> Vec<String> {
        let mut payloads = Vec::new();
        if !self.buffer.is_empty() {
            let line = String::from_utf8_lossy(&self.buffer)
                .trim_end_matches('\r')
                .to_string();
            self.buffer.clear();
            self.handle_line(&line, &mut payloads);
        }
        self.flush(&mut payloads);
        payloads
    }

    fn handle_line(&mut self, line: &str, payloads: &mut Vec<String>) {
        if line.is_empty() {
            self.flush(payloads);
            return;
        }
        if line.starts_with(':') {
            return;
        }
        if let Some(value) = line.strip_prefix("data:") {
            self.data_lines
                .push(value.strip_prefix(' ').unwrap_or(value).to_string());
        }
        // `event:` / `id:` / `retry:` 对 canonical 解码没有用，忽略。
    }

    fn flush(&mut self, payloads: &mut Vec<String>) {
        if self.data_lines.is_empty() {
            return;
        }
        let payload = self.data_lines.join("\n");
        self.data_lines.clear();
        if !payload.is_empty() {
            payloads.push(payload);
        }
    }
}

/// 把一帧 `data:` 载荷解码成 canonical 增量。
///
/// 返回 `Ok(None)` 表示该帧没有可用的增量（心跳、`[DONE]`、未知事件）。
pub fn decode_stream_event(protocol: LlmProtocol, data: &str) -> CoreResult<Option<ChatDelta>> {
    let data = data.trim();
    if data.is_empty() || data == "[DONE]" {
        return Ok(None);
    }
    let value: Value = serde_json::from_str(data)
        .map_err(|error| CoreError::InvalidParameter(format!("流式事件不是合法 JSON：{error}")))?;
    match protocol {
        LlmProtocol::OpenAiChat => decode_chat_event(&value),
        LlmProtocol::OpenAiResponses => decode_responses_event(&value),
        LlmProtocol::Anthropic => decode_anthropic_event(&value),
    }
}

fn decode_chat_event(value: &Value) -> CoreResult<Option<ChatDelta>> {
    let mut delta = ChatDelta::default();
    if let Some(choices) = value.get("choices").and_then(Value::as_array)
        && let Some(choice) = choices.first()
        && let Some(part) = choice.get("delta")
    {
        delta.content = part
            .get("content")
            .and_then(Value::as_str)
            .filter(|text| !text.is_empty())
            .map(str::to_string);
        delta.reasoning_content = part
            .get("reasoning_content")
            .and_then(Value::as_str)
            .filter(|text| !text.is_empty())
            .map(str::to_string);
        if let Some(reason) = choice.get("finish_reason").and_then(Value::as_str) {
            delta.finish_reason = Some(crate::openai::map_finish_reason(Some(reason)));
        }
    }
    if let Some(usage) = value.get("usage") {
        delta.usage = usage_from_openai(usage);
    }
    Ok(has_delta(delta))
}

fn decode_responses_event(value: &Value) -> CoreResult<Option<ChatDelta>> {
    let event_type = value
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let mut delta = ChatDelta::default();
    match event_type {
        "response.output_text.delta" => {
            delta.content = value
                .get("delta")
                .and_then(Value::as_str)
                .filter(|text| !text.is_empty())
                .map(str::to_string);
        }
        "response.reasoning_summary_text.delta" | "response.reasoning_text.delta" => {
            delta.reasoning_content = value
                .get("delta")
                .and_then(Value::as_str)
                .filter(|text| !text.is_empty())
                .map(str::to_string);
        }
        "response.completed" => {
            delta.finish_reason = Some(FinishReason::Stop);
            delta.usage = value
                .get("response")
                .and_then(|response| response.get("usage"))
                .and_then(usage_from_responses);
        }
        "response.incomplete" => {
            delta.finish_reason = Some(FinishReason::Length);
            delta.usage = value
                .get("response")
                .and_then(|response| response.get("usage"))
                .and_then(usage_from_responses);
        }
        "response.failed" | "error" => {
            let message = value
                .get("error")
                .and_then(|error| error.get("message"))
                .and_then(Value::as_str)
                .or_else(|| value.get("message").and_then(Value::as_str))
                .unwrap_or("Responses 流式调用失败");
            return Err(CoreError::InvalidParameter(message.to_string()));
        }
        _ => {}
    }
    Ok(has_delta(delta))
}

fn decode_anthropic_event(value: &Value) -> CoreResult<Option<ChatDelta>> {
    let event_type = value
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let mut delta = ChatDelta::default();
    match event_type {
        "message_start" => {
            delta.usage = value
                .get("message")
                .and_then(|message| message.get("usage"))
                .and_then(usage_from_anthropic);
        }
        "content_block_delta" => {
            if let Some(part) = value.get("delta") {
                match part.get("type").and_then(Value::as_str) {
                    Some("text_delta") => {
                        delta.content = part
                            .get("text")
                            .and_then(Value::as_str)
                            .filter(|text| !text.is_empty())
                            .map(str::to_string);
                    }
                    Some("thinking_delta") => {
                        delta.reasoning_content = part
                            .get("thinking")
                            .and_then(Value::as_str)
                            .filter(|text| !text.is_empty())
                            .map(str::to_string);
                    }
                    _ => {}
                }
            }
        }
        "message_delta" => {
            if let Some(reason) = value
                .get("delta")
                .and_then(|part| part.get("stop_reason"))
                .and_then(Value::as_str)
            {
                delta.finish_reason = Some(map_anthropic_stop(reason));
            }
            delta.usage = value.get("usage").and_then(usage_from_anthropic);
        }
        "error" => {
            let message = value
                .get("error")
                .and_then(|error| error.get("message"))
                .and_then(Value::as_str)
                .unwrap_or("Anthropic 流式调用失败");
            return Err(CoreError::InvalidParameter(message.to_string()));
        }
        _ => {}
    }
    Ok(has_delta(delta))
}

fn has_delta(delta: ChatDelta) -> Option<ChatDelta> {
    if delta.content.is_some()
        || delta.reasoning_content.is_some()
        || delta.finish_reason.is_some()
        || delta.usage.is_some()
    {
        Some(delta)
    } else {
        None
    }
}

fn usage_from_openai(usage: &Value) -> Option<TokenUsage> {
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
        .map(|v| v as u32)
        .unwrap_or(input + output);
    (input > 0 || output > 0 || total > 0).then_some(TokenUsage {
        input_tokens: input,
        output_tokens: output,
        total_tokens: total,
    })
}

fn usage_from_responses(usage: &Value) -> Option<TokenUsage> {
    let input = usage
        .get("input_tokens")
        .and_then(Value::as_u64)
        .unwrap_or(0) as u32;
    let output = usage
        .get("output_tokens")
        .and_then(Value::as_u64)
        .unwrap_or(0) as u32;
    let total = usage
        .get("total_tokens")
        .and_then(Value::as_u64)
        .map(|v| v as u32)
        .unwrap_or(input + output);
    (input > 0 || output > 0 || total > 0).then_some(TokenUsage {
        input_tokens: input,
        output_tokens: output,
        total_tokens: total,
    })
}

fn usage_from_anthropic(usage: &Value) -> Option<TokenUsage> {
    let input = usage
        .get("input_tokens")
        .and_then(Value::as_u64)
        .unwrap_or(0) as u32;
    let output = usage
        .get("output_tokens")
        .and_then(Value::as_u64)
        .unwrap_or(0) as u32;
    let total = input + output;
    (input > 0 || output > 0).then_some(TokenUsage {
        input_tokens: input,
        output_tokens: output,
        total_tokens: total,
    })
}

fn map_anthropic_stop(reason: &str) -> FinishReason {
    match reason {
        "end_turn" | "stop_sequence" => FinishReason::Stop,
        "max_tokens" => FinishReason::Length,
        "tool_use" => FinishReason::ToolCalls,
        "refusal" => FinishReason::ContentFilter,
        _ => FinishReason::Other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn sse_parser_handles_crlf_and_split_chunks() {
        let mut parser = SseParser::new();
        assert!(parser.feed(b"data: {\"a\":").is_empty());
        let payloads = parser.feed(b"1}\r\n\r\ndata: [DONE]\r\n\r\n");
        assert_eq!(
            payloads,
            vec!["{\"a\":1}".to_string(), "[DONE]".to_string()]
        );
    }

    #[test]
    fn chat_stream_decodes_text_reasoning_finish_and_usage() {
        let text = json!({
            "choices": [{"index": 0, "delta": {"content": "你", "reasoning_content": "想"}, "finish_reason": null}]
        });
        let delta = decode_stream_event(LlmProtocol::OpenAiChat, &text.to_string())
            .unwrap()
            .unwrap();
        assert_eq!(delta.content.as_deref(), Some("你"));
        assert_eq!(delta.reasoning_content.as_deref(), Some("想"));

        let done = json!({
            "choices": [{"index": 0, "delta": {}, "finish_reason": "stop"}],
            "usage": {"prompt_tokens": 3, "completion_tokens": 2, "total_tokens": 5}
        });
        let delta = decode_stream_event(LlmProtocol::OpenAiChat, &done.to_string())
            .unwrap()
            .unwrap();
        assert_eq!(delta.finish_reason, Some(FinishReason::Stop));
        assert_eq!(delta.usage.unwrap().total_tokens, 5);
    }

    #[test]
    fn responses_stream_decodes_text_and_completed_usage() {
        let text = json!({
            "type": "response.output_text.delta",
            "delta": "回答"
        });
        let delta = decode_stream_event(LlmProtocol::OpenAiResponses, &text.to_string())
            .unwrap()
            .unwrap();
        assert_eq!(delta.content.as_deref(), Some("回答"));

        let completed = json!({
            "type": "response.completed",
            "response": {"usage": {"input_tokens": 4, "output_tokens": 3, "total_tokens": 7}}
        });
        let delta = decode_stream_event(LlmProtocol::OpenAiResponses, &completed.to_string())
            .unwrap()
            .unwrap();
        assert_eq!(delta.finish_reason, Some(FinishReason::Stop));
        assert_eq!(delta.usage.unwrap().output_tokens, 3);
    }

    #[test]
    fn anthropic_stream_decodes_thinking_text_and_stop() {
        let thinking = json!({
            "type": "content_block_delta",
            "delta": {"type": "thinking_delta", "thinking": "推理"}
        });
        let delta = decode_stream_event(LlmProtocol::Anthropic, &thinking.to_string())
            .unwrap()
            .unwrap();
        assert_eq!(delta.reasoning_content.as_deref(), Some("推理"));

        let text = json!({
            "type": "content_block_delta",
            "delta": {"type": "text_delta", "text": "答案"}
        });
        let delta = decode_stream_event(LlmProtocol::Anthropic, &text.to_string())
            .unwrap()
            .unwrap();
        assert_eq!(delta.content.as_deref(), Some("答案"));

        let stop = json!({
            "type": "message_delta",
            "delta": {"stop_reason": "max_tokens"},
            "usage": {"output_tokens": 12}
        });
        let delta = decode_stream_event(LlmProtocol::Anthropic, &stop.to_string())
            .unwrap()
            .unwrap();
        assert_eq!(delta.finish_reason, Some(FinishReason::Length));
        assert_eq!(delta.usage.unwrap().output_tokens, 12);
    }
}
