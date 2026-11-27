//! 火山方舟（豆包）LLM 配置。
//!
//! `leo-plugins.json` → `"settings": {"leo-plugin-llm-volcengine": { ... }}`
//!
//! ```json
//! {
//!   "protocol": "openai-chat",
//!   "base_url": "https://ark.cn-beijing.volces.com/api/v3",
//!   "model": "ep-2026xxxxxxxx",
//!   "thinking": "enabled",
//!   "reasoning_effort": "high",
//!   "stream": true,
//!   "temperature": 0.7,
//!   "max_tokens": 4096
//! }
//! ```

use std::collections::BTreeMap;

use leo_core::error::{CoreError, CoreResult};
use leo_core::spi::{LlmProtocol, ReasoningEffort};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct VolcengineConfig {
    /// 读取 API Key 的环境变量名（Key 本身不写进配置文件）
    pub api_key_env: String,
    /// 上游协议：openai-chat / openai-responses / anthropic
    pub protocol: LlmProtocol,
    /// 协议根地址，不是完整 endpoint
    pub base_url: String,
    /// 推理接入点 ID（ep-...）或方舟模型 ID；没有通用默认值，通常必须配置
    pub model: String,
    /// 豆包思考模式：enabled / disabled / auto；不填不传
    pub thinking: Option<String>,
    /// 思考深度：none / minimal / low / medium / high / xhigh / max
    pub reasoning_effort: Option<ReasoningEffort>,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub max_tokens: Option<u32>,
    /// 默认是否走 SSE 流式；默认开启
    pub stream: bool,
    pub timeout_secs: u64,
    /// 额外请求头，例如网关要求的 `OpenAI-Beta: responses=v1`
    pub extra_headers: BTreeMap<String, String>,
    /// 额外请求体字段，原样合并进协议编码后的 JSON
    pub extra_body: BTreeMap<String, serde_json::Value>,
}

impl Default for VolcengineConfig {
    fn default() -> Self {
        Self {
            api_key_env: "ARK_API_KEY".into(),
            protocol: LlmProtocol::OpenAiChat,
            base_url: "https://ark.cn-beijing.volces.com/api/v3".into(),
            model: String::new(),
            thinking: None,
            reasoning_effort: None,
            temperature: None,
            top_p: None,
            max_tokens: None,
            stream: true,
            timeout_secs: 120,
            extra_headers: BTreeMap::new(),
            extra_body: BTreeMap::new(),
        }
    }
}

impl VolcengineConfig {
    pub fn from_settings(settings: Option<&serde_json::Value>) -> CoreResult<Self> {
        let config: Self = match settings {
            None => Self::default(),
            Some(value) => serde_json::from_value(value.clone()).map_err(|error| {
                CoreError::InvalidParameter(format!(
                    "leo-plugin-llm-volcengine 配置格式错误：{error}"
                ))
            })?,
        };
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> CoreResult<()> {
        let bad = |message: String| {
            Err(CoreError::InvalidParameter(format!(
                "leo-plugin-llm-volcengine 配置错误：{message}"
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
        if let Some(thinking) = &self.thinking
            && !["enabled", "disabled", "auto"].contains(&thinking.as_str())
        {
            return bad(format!(
                "thinking 只支持 enabled / disabled / auto，收到 {thinking}"
            ));
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
    fn defaults_are_ark_openai_chat() {
        let config = VolcengineConfig::default();
        assert_eq!(config.protocol, LlmProtocol::OpenAiChat);
        assert_eq!(config.base_url, "https://ark.cn-beijing.volces.com/api/v3");
        assert!(config.model.is_empty());
    }

    #[test]
    fn settings_reject_unknown_and_bad_values() {
        assert!(
            VolcengineConfig::from_settings(Some(&serde_json::json!({"protocal": "x"}))).is_err()
        );
        assert!(
            VolcengineConfig::from_settings(Some(&serde_json::json!({"timeout_secs": 0}))).is_err()
        );
        assert!(
            VolcengineConfig::from_settings(Some(&serde_json::json!({"thinking": "maybe"})))
                .is_err()
        );
        let ok = VolcengineConfig::from_settings(Some(&serde_json::json!({
            "model": "ep-2026xxxx",
            "thinking": "enabled"
        })))
        .unwrap();
        assert_eq!(ok.model, "ep-2026xxxx");
        assert_eq!(ok.thinking.as_deref(), Some("enabled"));
    }
}
