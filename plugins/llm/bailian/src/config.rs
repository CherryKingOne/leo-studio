//! 阿里云百炼（Qwen）LLM 配置。
//!
//! `leo-plugins.json` → `"settings": {"leo-plugin-llm-bailian": { ... }}`
//!
//! ```json
//! {
//!   "protocol": "openai-chat",
//!   "base_url": "https://dashscope.aliyuncs.com/compatible-mode/v1",
//!   "model": "qwen3.8-flash",
//!   "stream": true,
//!   "reasoning_effort": "high",
//!   "enable_search": false,
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
pub struct BailianConfig {
    /// 读取 API Key 的环境变量名（Key 本身不写进配置文件）
    pub api_key_env: String,
    /// 上游协议：openai-chat / openai-responses / anthropic
    pub protocol: LlmProtocol,
    /// 协议根地址，不是完整 endpoint
    pub base_url: String,
    /// 请求没指定模型时用的模型
    pub model: String,
    /// Qwen3 等模型是否开启思考模式；不填不传
    pub enable_thinking: Option<bool>,
    /// 思考深度：none / minimal / low / medium / high / xhigh / max；与 thinking_budget 二选一
    pub reasoning_effort: Option<ReasoningEffort>,
    /// 思考过程最大 token 数；与 reasoning_effort 二选一
    pub thinking_budget: Option<u32>,
    /// 是否开启联网搜索；不填不传
    pub enable_search: Option<bool>,
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

impl Default for BailianConfig {
    fn default() -> Self {
        Self {
            api_key_env: "DASHSCOPE_API_KEY".into(),
            protocol: LlmProtocol::OpenAiChat,
            base_url: "https://dashscope.aliyuncs.com/compatible-mode/v1".into(),
            model: "qwen-plus".into(),
            enable_thinking: None,
            reasoning_effort: None,
            thinking_budget: None,
            enable_search: None,
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

impl BailianConfig {
    pub fn from_settings(settings: Option<&serde_json::Value>) -> CoreResult<Self> {
        let config: Self = match settings {
            None => Self::default(),
            Some(value) => serde_json::from_value(value.clone()).map_err(|error| {
                CoreError::InvalidParameter(format!("leo-plugin-llm-bailian 配置格式错误：{error}"))
            })?,
        };
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> CoreResult<()> {
        let bad = |message: String| {
            Err(CoreError::InvalidParameter(format!(
                "leo-plugin-llm-bailian 配置错误：{message}"
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
        if self.reasoning_effort.is_some() && self.thinking_budget.is_some() {
            return bad(
                "reasoning_effort 与 thinking_budget 不能同时设置（Qwen3.8 / Qwen4 会直接报错）"
                    .into(),
            );
        }
        if let Some(budget) = self.thinking_budget
            && budget == 0
        {
            return bad("thinking_budget 必须大于 0".into());
        }
        if self.enable_thinking == Some(false)
            && (self.reasoning_effort.is_some() || self.thinking_budget.is_some())
        {
            return bad("enable_thinking=false 与思考深度 / 思考预算冲突".into());
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
    fn defaults_are_dashscope_compatible_mode() {
        let config = BailianConfig::default();
        assert_eq!(config.protocol, LlmProtocol::OpenAiChat);
        assert_eq!(
            config.base_url,
            "https://dashscope.aliyuncs.com/compatible-mode/v1"
        );
        assert_eq!(config.model, "qwen-plus");
    }

    #[test]
    fn settings_reject_unknown_and_bad_values() {
        assert!(BailianConfig::from_settings(Some(&serde_json::json!({"protocal": "x"}))).is_err());
        assert!(
            BailianConfig::from_settings(Some(&serde_json::json!({"timeout_secs": 0}))).is_err()
        );
        assert!(
            BailianConfig::from_settings(Some(&serde_json::json!({"temperature": -1.0}))).is_err()
        );
        let ok = BailianConfig::from_settings(Some(&serde_json::json!({
            "model": "qwen-max",
            "enable_thinking": true,
            "enable_search": false
        })))
        .unwrap();
        assert_eq!(ok.model, "qwen-max");
        assert_eq!(ok.enable_thinking, Some(true));
        assert_eq!(ok.enable_search, Some(false));
    }
}
