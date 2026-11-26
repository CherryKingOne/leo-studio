//! DeepSeek LLM 配置。
//!
//! `leo-plugins.json` → `"settings": {"leo-plugin-llm-deepseek": { ... }}`
//!
//! ```json
//! {
//!   "protocol": "openai-chat",
//!   "base_url": "https://api.deepseek.com/v1",
//!   "model": "deepseek-chat",
//!   "temperature": 0.7,
//!   "max_tokens": 4096
//! }
//! ```

use std::collections::BTreeMap;

use leo_core::error::{CoreError, CoreResult};
use leo_core::spi::LlmProtocol;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DeepseekConfig {
    /// 读取 API Key 的环境变量名（Key 本身不写进配置文件）
    pub api_key_env: String,
    /// 上游协议：openai-chat / openai-responses / anthropic
    pub protocol: LlmProtocol,
    /// 协议根地址，不是完整 endpoint；路径由 protocol 决定
    pub base_url: String,
    /// 请求没指定模型时用的模型
    pub model: String,
    /// 默认温度；请求里没给时使用
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub max_tokens: Option<u32>,
    /// 单次请求超时（秒）
    pub timeout_secs: u64,
    /// 额外请求头，例如网关要求的 `OpenAI-Beta: responses=v1`
    pub extra_headers: BTreeMap<String, String>,
    /// 额外请求体字段，原样合并进协议编码后的 JSON，例如 `{"thinking":{"type":"enabled"}}`
    pub extra_body: BTreeMap<String, serde_json::Value>,
}

impl Default for DeepseekConfig {
    fn default() -> Self {
        Self {
            api_key_env: "DEEPSEEK_API_KEY".into(),
            protocol: LlmProtocol::OpenAiChat,
            base_url: "https://api.deepseek.com/v1".into(),
            model: "deepseek-chat".into(),
            temperature: None,
            top_p: None,
            max_tokens: None,
            timeout_secs: 120,
            extra_headers: BTreeMap::new(),
            extra_body: BTreeMap::new(),
        }
    }
}

impl DeepseekConfig {
    pub fn from_settings(settings: Option<&serde_json::Value>) -> CoreResult<Self> {
        let config: Self = match settings {
            None => Self::default(),
            Some(value) => serde_json::from_value(value.clone()).map_err(|error| {
                CoreError::InvalidParameter(format!(
                    "leo-plugin-llm-deepseek 配置格式错误：{error}"
                ))
            })?,
        };
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> CoreResult<()> {
        let bad = |message: String| {
            Err(CoreError::InvalidParameter(format!(
                "leo-plugin-llm-deepseek 配置错误：{message}"
            )))
        };
        if self.base_url.trim().is_empty() {
            return bad("base_url 不能为空".into());
        }
        if !self.base_url.starts_with("http://") && !self.base_url.starts_with("https://") {
            return bad(format!(
                "base_url 必须以 http:// 或 https:// 开头，收到 {}",
                self.base_url
            ));
        }
        if self.timeout_secs == 0 {
            return bad("timeout_secs 必须大于 0".into());
        }
        if let Some(temperature) = self.temperature
            && !(0.0..=2.0).contains(&temperature)
        {
            return bad(format!("temperature 范围 0.0 ~ 2.0，收到 {temperature}"));
        }
        if let Some(top_p) = self.top_p
            && !(0.0..=1.0).contains(&top_p)
        {
            return bad(format!("top_p 范围 0.0 ~ 1.0，收到 {top_p}"));
        }
        if let Some(max_tokens) = self.max_tokens
            && max_tokens == 0
        {
            return bad("max_tokens 必须大于 0".into());
        }
        if self.extra_headers.keys().any(|key| key.trim().is_empty()) {
            return bad("extra_headers 里不能有空 header 名".into());
        }
        Ok(())
    }

    /// 从环境变量读 API Key
    pub fn api_key(&self) -> Option<String> {
        std::env::var(&self.api_key_env)
            .ok()
            .map(|key| key.trim().to_string())
            .filter(|key| !key.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_openai_chat() {
        let config = DeepseekConfig::default();
        assert_eq!(config.protocol, LlmProtocol::OpenAiChat);
        assert_eq!(config.base_url, "https://api.deepseek.com/v1");
        assert_eq!(config.model, "deepseek-chat");
    }

    #[test]
    fn settings_reject_unknown_and_bad_values() {
        assert!(
            DeepseekConfig::from_settings(Some(&serde_json::json!({"protocal": "x"}))).is_err()
        );
        assert!(
            DeepseekConfig::from_settings(Some(&serde_json::json!({"timeout_secs": 0}))).is_err()
        );
        assert!(
            DeepseekConfig::from_settings(Some(&serde_json::json!({"temperature": 3.0}))).is_err()
        );
        assert!(
            DeepseekConfig::from_settings(Some(&serde_json::json!({"base_url": "ftp://x"})))
                .is_err()
        );
        let ok = DeepseekConfig::from_settings(Some(&serde_json::json!({
            "protocol": "anthropic",
            "base_url": "https://api.deepseek.com/anthropic",
            "model": "deepseek-chat"
        })))
        .unwrap();
        assert_eq!(ok.protocol, LlmProtocol::Anthropic);
    }
}
