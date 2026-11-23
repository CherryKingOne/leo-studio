//! DeepSeek 配置。
//!
//! 参数含义与取值范围以官方文档为准：
//! - Chat Completions：<https://api-docs.deepseek.com/api/create-chat-completion>
//! - Responses：<https://api-docs.deepseek.com/api/create-response>
//! - 思考模式：<https://api-docs.deepseek.com/guides/thinking_mode>

use leo_core::error::{CoreError, CoreResult};
use leo_core::spi::LlmApi;
use serde::{Deserialize, Serialize};

/// `leo-plugins.json` → `"settings": {"leo-plugin-llm-deepseek": { ... }}`
///
/// ```json
/// {
///   "api": "chat_completions",
///   "model": "deepseek-flash",
///   "temperature": 0.2,
///   "max_tokens": 4096,
///   "thinking": false
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DeepSeekConfig {
    /// 读取 API Key 的环境变量名（Key 本身不写进配置文件）
    pub api_key_env: String,
    /// 接口地址（OpenAI 兼容格式），请求发往 `{base_url}/chat/completions` 或 `{base_url}/responses`
    pub base_url: String,
    /// 默认接口风格：`chat_completions`（默认）/ `responses`；请求里的 `api` 优先
    pub api: LlmApi,
    /// 默认模型：deepseek-flash / deepseek-v4-pro（请求里的 model 优先）
    pub model: String,
    /// 采样温度 [0, 2]；不填不传（官方默认 1）。思考模式下无效
    pub temperature: Option<f32>,
    /// 最大输出 token [1, 393216]；不填不传
    pub max_tokens: Option<u32>,
    /// 思考模式：false 关闭（默认）、true 开启、null 不传（官方默认开启）
    pub thinking: Option<bool>,
    /// 思考强度：low / high / max；不填不传（官方默认 high）。只在思考模式下有意义，thinking=false 时不能填
    pub reasoning_effort: Option<ReasoningEffort>,
    /// 单次请求超时（秒）
    pub timeout_secs: u64,
    /// 429 / 5xx / 网络错误时的最大重试次数
    pub max_retries: u32,
}

impl Default for DeepSeekConfig {
    fn default() -> Self {
        Self {
            api_key_env: "DEEPSEEK_API_KEY".into(),
            base_url: "https://api.deepseek.com".into(),
            api: LlmApi::ChatCompletions,
            model: "deepseek-flash".into(),
            temperature: None,
            max_tokens: None,
            thinking: Some(false),
            reasoning_effort: None,
            timeout_secs: 120,
            max_retries: 2,
        }
    }
}

/// 思考强度（官方：Chat Completions 用 `reasoning_effort`，Responses 用 `reasoning.effort`）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReasoningEffort {
    Low,
    High,
    Max,
}

impl ReasoningEffort {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::High => "high",
            Self::Max => "max",
        }
    }
}

/// 官方文档：max_tokens 取值 1 ~ 384K
pub const MAX_TOKENS_LIMIT: u32 = 393_216;
/// 重试次数上限，防止配置写错导致长时间重试
pub const MAX_RETRIES_LIMIT: u32 = 10;

impl DeepSeekConfig {
    pub fn from_settings(settings: Option<&serde_json::Value>) -> CoreResult<Self> {
        let config: Self = match settings {
            None => Self::default(),
            Some(v) => serde_json::from_value(v.clone()).map_err(|e| {
                CoreError::InvalidParameter(format!("leo-plugin-llm-deepseek 配置格式错误：{e}"))
            })?,
        };
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> CoreResult<()> {
        let bad = |m: String| Err(CoreError::InvalidParameter(format!("leo-plugin-llm-deepseek 配置错误：{m}")));
        if self.api_key_env.trim().is_empty() {
            return bad("api_key_env 不能为空".into());
        }
        if !(self.base_url.starts_with("http://") || self.base_url.starts_with("https://")) {
            return bad(format!("base_url 需要以 http:// 或 https:// 开头，收到 {}", self.base_url));
        }
        if self.model.trim().is_empty() {
            return bad("model 不能为空".into());
        }
        if let Some(t) = self.temperature {
            check_temperature(t).or_else(bad)?;
        }
        if let Some(n) = self.max_tokens {
            check_max_tokens(n).or_else(bad)?;
        }
        if self.thinking == Some(false) && self.reasoning_effort.is_some() {
            return bad("reasoning_effort 只在思考模式下生效，thinking=false 时不要填写".into());
        }
        if self.timeout_secs == 0 {
            return bad("timeout_secs 必须大于 0".into());
        }
        if self.max_retries > MAX_RETRIES_LIMIT {
            return bad(format!("max_retries 最大 {MAX_RETRIES_LIMIT}，收到 {}", self.max_retries));
        }
        Ok(())
    }

    /// 从环境变量读 API Key
    pub fn api_key(&self) -> Option<String> {
        std::env::var(&self.api_key_env).ok().map(|k| k.trim().to_string()).filter(|k| !k.is_empty())
    }
}

pub(crate) fn check_temperature(t: f32) -> Result<(), String> {
    if (0.0..=2.0).contains(&t) { Ok(()) } else { Err(format!("temperature 范围 [0, 2]，收到 {t}")) }
}

pub(crate) fn check_max_tokens(n: u32) -> Result<(), String> {
    if (1..=MAX_TOKENS_LIMIT).contains(&n) {
        Ok(())
    } else {
        Err(format!("max_tokens 范围 [1, {MAX_TOKENS_LIMIT}]，收到 {n}"))
    }
}
