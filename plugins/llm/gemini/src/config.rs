//! LLM 供应商通用配置。
//!
//! `leo-plugins.json` → `"settings": {"<plugin-id>": { ... }}`
//!
//! 所有供应商共用同一套字段；默认值由各 crate 的 `lib.rs` 常量和
//! `models.rs` 决定。API Key 只从环境变量读取，不写进配置文件。

use std::collections::BTreeMap;

use leo_core::error::{CoreError, CoreResult};
use leo_core::spi::{LlmProtocol, ThinkingConfig};
use serde::{Deserialize, Serialize};

use crate::{
    API_KEY_ENV_ALIASES, DEFAULT_API_KEY_ENV, DEFAULT_BASE_URL, DEFAULT_MODEL, DEFAULT_MODELS_URL,
    DEFAULT_REASONING_EFFORT, DEFAULT_THINKING_MODE, PLUGIN_ID, SUPPORTED_PROTOCOLS,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    /// 读取 API Key 的环境变量名（Key 本身不写进配置文件）
    pub api_key_env: String,
    /// 备用环境变量名；主变量没有值时按顺序读取
    pub api_key_env_aliases: Vec<String>,
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
    /// 默认是否走 SSE 流式；默认开启
    pub stream: bool,
    /// 默认思考模式 / 思考深度
    pub thinking: ThinkingConfig,
    /// 单次请求超时（秒）
    pub timeout_secs: u64,
    /// 额外请求头，例如网关要求的 `OpenAI-Beta: responses=v1`
    pub extra_headers: BTreeMap<String, String>,
    /// 额外请求体字段，原样合并进协议编码后的 JSON
    pub extra_body: BTreeMap<String, serde_json::Value>,
    /// 远端模型列表地址；没有统一 `/models` 的厂商默认不填
    pub models_url: Option<String>,
    /// 远端模型列表缓存秒数；0 表示不缓存
    pub models_cache_secs: u64,
    /// 允许的自定义 / 私有化模型 id，用于补充前缀白名单
    pub allowed_models: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            api_key_env: DEFAULT_API_KEY_ENV.into(),
            api_key_env_aliases: API_KEY_ENV_ALIASES
                .iter()
                .map(|name| (*name).to_string())
                .collect(),
            protocol: LlmProtocol::OpenAiChat,
            base_url: DEFAULT_BASE_URL.into(),
            model: DEFAULT_MODEL.into(),
            temperature: None,
            top_p: None,
            max_tokens: None,
            stream: true,
            thinking: ThinkingConfig {
                mode: DEFAULT_THINKING_MODE,
                effort: DEFAULT_REASONING_EFFORT,
                budget_tokens: None,
            },
            timeout_secs: 120,
            extra_headers: BTreeMap::new(),
            extra_body: BTreeMap::new(),
            models_url: DEFAULT_MODELS_URL.map(str::to_string),
            models_cache_secs: 600,
            allowed_models: Vec::new(),
        }
    }
}

impl Config {
    pub fn from_settings(settings: Option<&serde_json::Value>) -> CoreResult<Self> {
        let config: Self = match settings {
            None => Self::default(),
            Some(value) => serde_json::from_value(value.clone()).map_err(|error| {
                CoreError::InvalidParameter(format!("{PLUGIN_ID} 配置格式错误：{error}"))
            })?,
        };
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> CoreResult<()> {
        let bad = |message: String| {
            Err(CoreError::InvalidParameter(format!(
                "{PLUGIN_ID} 配置错误：{message}"
            )))
        };
        if self.api_key_env.trim().is_empty() {
            return bad("api_key_env 不能为空".into());
        }
        if self
            .api_key_env_aliases
            .iter()
            .any(|name| name.trim().is_empty())
        {
            return bad("api_key_env_aliases 里不能有空变量名".into());
        }
        if !SUPPORTED_PROTOCOLS.contains(&self.protocol) {
            let supported = SUPPORTED_PROTOCOLS
                .iter()
                .map(|protocol| protocol.as_str())
                .collect::<Vec<_>>()
                .join(" / ");
            return bad(format!(
                "协议 {} 不受支持；本插件支持：{supported}",
                self.protocol.as_str()
            ));
        }
        if self.base_url.trim().is_empty() {
            return bad("base_url 不能为空".into());
        }
        if !self.base_url.starts_with("http://") && !self.base_url.starts_with("https://") {
            return bad(format!(
                "base_url 必须以 http:// 或 https:// 开头，收到 {}",
                self.base_url
            ));
        }
        if let Some(models_url) = &self.models_url {
            if models_url.trim().is_empty() {
                return bad("models_url 不能为空字符串；不需要远端列表请不填".into());
            }
            if !models_url.starts_with("http://") && !models_url.starts_with("https://") {
                return bad(format!(
                    "models_url 必须以 http:// 或 https:// 开头，收到 {models_url}"
                ));
            }
        }
        if self.model.trim().is_empty() {
            return bad("model 不能为空".into());
        }
        if !crate::models::is_owned_model(&self.model, &self.allowed_models) {
            return bad(format!(
                "默认模型 `{}` 不属于本插件的自有模型范围；可用模型见 models.rs，或把自定义模型写进 allowed_models",
                self.model
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
        if self
            .allowed_models
            .iter()
            .any(|model| model.trim().is_empty())
        {
            return bad("allowed_models 里不能有空模型 id".into());
        }
        Ok(())
    }

    /// 从环境变量读 API Key：主变量优先，再按顺序读备用变量。
    pub fn api_key(&self) -> Option<String> {
        for env_name in std::iter::once(&self.api_key_env).chain(self.api_key_env_aliases.iter()) {
            if let Ok(key) = std::env::var(env_name) {
                let key = key.trim().to_string();
                if !key.is_empty() {
                    return Some(key);
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_valid() {
        let config = Config::default();
        assert_eq!(config.protocol, LlmProtocol::OpenAiChat);
        assert_eq!(config.model, crate::DEFAULT_MODEL);
        config.validate().unwrap();
    }

    #[test]
    fn unsupported_protocol_is_rejected() {
        if let Some(protocol) = [
            LlmProtocol::OpenAiChat,
            LlmProtocol::OpenAiResponses,
            LlmProtocol::Anthropic,
        ]
        .into_iter()
        .find(|protocol| !SUPPORTED_PROTOCOLS.contains(protocol))
        {
            let config = Config {
                protocol,
                ..Default::default()
            };
            assert!(config.validate().is_err());
        }
    }

    #[test]
    fn foreign_default_model_is_rejected() {
        let config = Config {
            model: "definitely-not-this-vendor".into(),
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }
}
