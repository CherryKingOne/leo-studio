//! Leo Studio LLM 共享协议库。
//!
//! 这个 crate 不是插件：没有 `Plugin` 实现、不注册驱动、不进 CATALOG。
//! 它只做三件事：
//!
//! 1. 把 canonical [`ChatRequest`] / [`ChatResponse`] 编码成三种上游协议，或反过来解码；
//! 2. 提供协议对应的路径与鉴权头；
//! 3. 提供跨协议转换（canonical 作为中转）。
//!
//! 支持的上游协议：
//! - [`LlmProtocol::OpenAiChat`]：OpenAI 旧接口 `POST /chat/completions`
//! - [`LlmProtocol::OpenAiResponses`]：OpenAI 新接口 `POST /responses`
//! - [`LlmProtocol::Anthropic`]：Anthropic Messages `POST /v1/messages`
//!
//! 供应商子插件只依赖本库和 `leo-core`，不需要自己拼三家协议的请求体。

pub mod anthropic;
pub mod convert;
pub mod endpoint;
pub mod error;
pub mod openai;
pub mod stream;
pub mod thinking;

pub use stream::{SseParser, decode_stream_event};

pub use endpoint::ANTHROPIC_VERSION;
use leo_core::error::{CoreError, CoreResult};
pub use leo_core::spi::LlmProtocol;
use leo_core::spi::{ChatRequest, ChatResponse};

/// 把 canonical 请求编码成指定协议的 JSON 请求体。
///
/// `model` 由调用方决定：供应商插件通常传配置里的默认模型，注册层的协议桥则传请求里的模型。
pub fn encode_request(
    protocol: LlmProtocol,
    request: &ChatRequest,
    model: &str,
) -> serde_json::Value {
    match protocol {
        LlmProtocol::OpenAiChat => openai::chat_completions::encode_request(request, model),
        LlmProtocol::OpenAiResponses => openai::responses::encode_request(request, model),
        LlmProtocol::Anthropic => anthropic::messages::encode_request(request, model),
    }
}

/// 把指定协议的 JSON 请求体解码成 canonical 请求。
pub fn decode_request(protocol: LlmProtocol, body: &serde_json::Value) -> CoreResult<ChatRequest> {
    match protocol {
        LlmProtocol::OpenAiChat => openai::chat_completions::decode_request(body),
        LlmProtocol::OpenAiResponses => openai::responses::decode_request(body),
        LlmProtocol::Anthropic => anthropic::messages::decode_request(body),
    }
}

/// 把 canonical 响应编码成指定协议的 JSON 响应体。
pub fn encode_response(protocol: LlmProtocol, response: &ChatResponse) -> serde_json::Value {
    match protocol {
        LlmProtocol::OpenAiChat => openai::chat_completions::encode_response(response),
        LlmProtocol::OpenAiResponses => openai::responses::encode_response(response),
        LlmProtocol::Anthropic => anthropic::messages::encode_response(response),
    }
}

/// 把指定协议的 JSON 响应体解码成 canonical 响应。
pub fn decode_response(
    protocol: LlmProtocol,
    body: &serde_json::Value,
) -> CoreResult<ChatResponse> {
    match protocol {
        LlmProtocol::OpenAiChat => openai::chat_completions::decode_response(body),
        LlmProtocol::OpenAiResponses => openai::responses::decode_response(body),
        LlmProtocol::Anthropic => anthropic::messages::decode_response(body),
    }
}

/// 按协议拼出完整接口地址。`base_url` 是协议根地址，不是完整 endpoint。
pub fn endpoint(base_url: &str, protocol: LlmProtocol) -> String {
    endpoint::url(base_url, protocol)
}

/// 按协议生成鉴权头。
pub fn auth_headers(protocol: LlmProtocol, api_key: &str) -> Vec<(String, String)> {
    endpoint::auth_headers(protocol, api_key)
}

/// 把 HTTP 错误响应体解析成统一的 [`CoreError`]。
pub fn decode_error(plugin_id: &str, protocol: LlmProtocol, status: u16, body: &str) -> CoreError {
    error::decode_error(plugin_id, protocol, status, body)
}

/// 解析本次请求使用的模型：请求里显式指定的优先，其次用插件配置的默认模型。
pub fn resolve_model<'a>(
    request_model: Option<&'a str>,
    configured: &'a str,
) -> CoreResult<&'a str> {
    if let Some(model) = request_model.map(str::trim).filter(|m| !m.is_empty()) {
        return Ok(model);
    }
    if !configured.trim().is_empty() {
        return Ok(configured.trim());
    }
    Err(CoreError::InvalidParameter(
        "LLM 请求没有指定模型，且插件配置里也没有默认 model".into(),
    ))
}
