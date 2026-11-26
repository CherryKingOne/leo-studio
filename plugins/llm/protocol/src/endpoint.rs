//! 协议对应的接口路径与鉴权头。
//!
//! `base_url` 约定为**协议根地址**，不是完整 endpoint：
//! - `openai-chat` → `{base_url}/chat/completions`
//! - `openai-responses` → `{base_url}/responses`
//! - `anthropic` → `{base_url}/v1/messages`
//!
//! 各家 OpenAI 兼容地址通常已经带 `/v1` 或 `/api/v3`，所以路径里不再重复版本段；
//! Anthropic 的版本段 `/v1` 属于 Messages 接口本身，保留在路径里。

use leo_core::spi::LlmProtocol;

pub const ANTHROPIC_VERSION: &str = "2023-06-01";

/// 协议在 `base_url` 之后追加的路径
pub fn path(protocol: LlmProtocol) -> &'static str {
    match protocol {
        LlmProtocol::OpenAiChat => "/chat/completions",
        LlmProtocol::OpenAiResponses => "/responses",
        LlmProtocol::Anthropic => "/v1/messages",
    }
}

/// 拼出完整接口地址
pub fn url(base_url: &str, protocol: LlmProtocol) -> String {
    format!("{}{}", base_url.trim_end_matches('/'), path(protocol))
}

/// 按协议生成鉴权头；返回 `(header_name, header_value)`。
pub fn auth_headers(protocol: LlmProtocol, api_key: &str) -> Vec<(String, String)> {
    match protocol {
        LlmProtocol::OpenAiChat | LlmProtocol::OpenAiResponses => {
            vec![("Authorization".into(), format!("Bearer {api_key}"))]
        }
        LlmProtocol::Anthropic => vec![
            ("x-api-key".into(), api_key.into()),
            ("anthropic-version".into(), ANTHROPIC_VERSION.into()),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_and_urls() {
        assert_eq!(
            url("https://api.deepseek.com/v1/", LlmProtocol::OpenAiChat),
            "https://api.deepseek.com/v1/chat/completions"
        );
        assert_eq!(
            url("https://gateway/v1", LlmProtocol::OpenAiResponses),
            "https://gateway/v1/responses"
        );
        assert_eq!(
            url("https://api.deepseek.com/anthropic", LlmProtocol::Anthropic),
            "https://api.deepseek.com/anthropic/v1/messages"
        );
    }

    #[test]
    fn auth_headers_differ_by_protocol() {
        let openai = auth_headers(LlmProtocol::OpenAiChat, "k");
        assert_eq!(openai, vec![("Authorization".into(), "Bearer k".into())]);
        let anthropic = auth_headers(LlmProtocol::Anthropic, "k");
        assert_eq!(anthropic[0], ("x-api-key".to_string(), "k".to_string()));
        assert_eq!(
            anthropic[1],
            (
                "anthropic-version".to_string(),
                ANTHROPIC_VERSION.to_string()
            )
        );
    }
}
