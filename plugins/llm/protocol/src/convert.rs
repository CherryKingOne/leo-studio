//! 跨协议转换：入协议 → canonical → 出协议。
//!
//! 这是“支持转化”的统一入口。直接两两转换会写出 N×N 套映射；走 canonical 后
//! 只需要 N 套 decode + N 套 encode，新增协议只加一个模块。
//!
//! 有损点集中在这里记录：
//! - Chat Completions 的 `stop` 在 Responses 没有原生字段，转换时丢失
//! - Anthropic 没有 `response_format`，JSON 要求降级成 system 指令
//! - 三家的 usage 字段名不同，统一成 input / output / total

use leo_core::error::{CoreError, CoreResult};
use leo_core::spi::LlmProtocol;
use serde_json::Value;

/// 请求跨协议转换
pub fn convert_request(from: LlmProtocol, to: LlmProtocol, body: &Value) -> CoreResult<Value> {
    if from == to {
        return Ok(body.clone());
    }
    let request = crate::decode_request(from, body)?;
    let model = request
        .model
        .as_deref()
        .map(str::trim)
        .filter(|model| !model.is_empty())
        .ok_or_else(|| CoreError::InvalidParameter("跨协议转换的请求缺少 model 字段".into()))?;
    Ok(crate::encode_request(to, &request, model))
}

/// 响应跨协议转换
pub fn convert_response(from: LlmProtocol, to: LlmProtocol, body: &Value) -> CoreResult<Value> {
    if from == to {
        return Ok(body.clone());
    }
    let response = crate::decode_response(from, body)?;
    Ok(crate::encode_response(to, &response))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn chat_request_to_anthropic_keeps_system_and_merges_roles() {
        let chat = json!({
            "model": "deepseek-chat",
            "messages": [
                {"role": "system", "content": "你是校对助手"},
                {"role": "user", "content": "第一段"},
                {"role": "user", "content": "第二段"}
            ],
            "max_tokens": 2048,
            "temperature": 0.3
        });
        let anthropic =
            convert_request(LlmProtocol::OpenAiChat, LlmProtocol::Anthropic, &chat).unwrap();
        assert_eq!(anthropic["model"], json!("deepseek-chat"));
        assert_eq!(anthropic["system"], json!("你是校对助手"));
        assert_eq!(anthropic["messages"].as_array().unwrap().len(), 1);
        assert_eq!(
            anthropic["messages"][0]["content"],
            json!("第一段\n\n第二段")
        );
        assert_eq!(anthropic["max_tokens"], json!(2048));
    }

    #[test]
    fn chat_request_to_responses_maps_instructions_and_input() {
        let chat = json!({
            "model": "gpt-4.1",
            "messages": [
                {"role": "system", "content": "遵守格式"},
                {"role": "user", "content": "你好"}
            ],
            "max_tokens": 512,
            "response_format": {"type": "json_object"}
        });
        let responses =
            convert_request(LlmProtocol::OpenAiChat, LlmProtocol::OpenAiResponses, &chat).unwrap();
        assert_eq!(responses["instructions"], json!("遵守格式"));
        assert_eq!(responses["input"][0]["role"], json!("user"));
        assert_eq!(responses["max_output_tokens"], json!(512));
        assert_eq!(responses["text"]["format"]["type"], json!("json_object"));
    }

    #[test]
    fn anthropic_response_to_chat_response() {
        let anthropic = json!({
            "id": "msg_1",
            "model": "claude-sonnet",
            "type": "message",
            "role": "assistant",
            "content": [{"type": "text", "text": "答案"}],
            "stop_reason": "end_turn",
            "usage": {"input_tokens": 4, "output_tokens": 6}
        });
        let chat =
            convert_response(LlmProtocol::Anthropic, LlmProtocol::OpenAiChat, &anthropic).unwrap();
        assert_eq!(chat["choices"][0]["message"]["content"], json!("答案"));
        assert_eq!(chat["usage"]["prompt_tokens"], json!(4));
        assert_eq!(chat["usage"]["completion_tokens"], json!(6));
        assert_eq!(chat["usage"]["total_tokens"], json!(10));
    }

    #[test]
    fn responses_response_to_anthropic_response() {
        let responses = json!({
            "id": "resp_1",
            "model": "gpt-4.1",
            "status": "completed",
            "output": [{"type": "message", "role": "assistant", "content": [{"type": "output_text", "text": "答案"}]}],
            "usage": {"input_tokens": 5, "output_tokens": 7, "total_tokens": 12}
        });
        let anthropic = convert_response(
            LlmProtocol::OpenAiResponses,
            LlmProtocol::Anthropic,
            &responses,
        )
        .unwrap();
        assert_eq!(anthropic["content"][0]["text"], json!("答案"));
        assert_eq!(anthropic["usage"]["input_tokens"], json!(5));
        assert_eq!(anthropic["usage"]["output_tokens"], json!(7));
    }
}
