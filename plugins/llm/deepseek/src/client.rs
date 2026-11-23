//! DeepSeek HTTP 接口调用。
//!
//! - 对话：`POST {base_url}/chat/completions`（OpenAI 兼容，非流式），Bearer 鉴权
//!   <https://api-docs.deepseek.com/api/create-chat-completion>
//! - JSON 输出：`response_format = {"type": "json_object"}`，提示词里需要写明要 JSON
//!   <https://api-docs.deepseek.com/guides/json_mode>
//! - 错误码：<https://api-docs.deepseek.com/quick_start/error_codes>（429 / 500 / 503 可重试）

use std::time::Duration;

use async_trait::async_trait;
use leo_core::error::{CoreError, CoreResult};
use leo_core::spi::{ChatRequest, ChatResponse, ChatRole, LlmProvider, ResponseFormat, TokenUsage};
use serde::Deserialize;
use serde_json::{Map, Value, json};

use crate::config::{DeepSeekConfig, check_max_tokens, check_temperature};
use crate::{DRIVER_ID, PLUGIN_ID};

fn plugin_err(message: impl Into<String>) -> CoreError {
    CoreError::Plugin { plugin_id: PLUGIN_ID.into(), message: message.into() }
}

pub struct DeepSeekLlmProvider {
    config: DeepSeekConfig,
    /// 显式传入的 Key；为空时每次调用从环境变量读取
    api_key: Option<String>,
    http: reqwest::Client,
    backoff_base: Duration,
}

#[derive(Deserialize)]
struct CompletionResp {
    #[serde(default)]
    model: String,
    #[serde(default)]
    choices: Vec<Choice>,
    #[serde(default)]
    usage: Option<Usage>,
}

#[derive(Deserialize)]
struct Choice {
    message: Message,
    #[serde(default)]
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct Message {
    #[serde(default)]
    content: Option<String>,
}

#[derive(Deserialize)]
struct Usage {
    #[serde(default)]
    prompt_tokens: u32,
    #[serde(default)]
    completion_tokens: u32,
    #[serde(default)]
    total_tokens: u32,
}

#[derive(Deserialize)]
struct ErrorResp {
    error: ErrorBody,
}

#[derive(Deserialize)]
struct ErrorBody {
    #[serde(default)]
    message: String,
}

/// 是否值得重试：429 限流、5xx 服务端错误
pub fn is_retryable(status: u16) -> bool {
    status == 429 || (500..=599).contains(&status)
}

/// HTTP 错误 → 带说明的错误
pub fn map_http_error(status: u16, body: &str) -> CoreError {
    plugin_err(http_error_message(status, body))
}

fn http_error_message(status: u16, body: &str) -> String {
    let detail = serde_json::from_str::<ErrorResp>(body)
        .map(|e| e.error.message)
        .ok()
        .filter(|m| !m.is_empty())
        .unwrap_or_else(|| body.chars().take(300).collect());
    let hint = match status {
        400 => "请求格式错误（400），请按错误信息修改请求体",
        401 => "鉴权失败（401），API Key 错误，请检查 DeepSeek API Key：https://platform.deepseek.com/api_keys",
        402 => "余额不足（402），请到 https://platform.deepseek.com/top_up 充值",
        422 => "参数错误（422），请按错误信息修改请求参数（如 model、max_tokens、temperature）",
        429 => "请求过于频繁（429），已达速率或并发上限",
        500 => "服务端错误（500）",
        503 => "服务器繁忙（503）",
        s if (500..=599).contains(&s) => "服务端错误",
        _ => "请求失败",
    };
    format!("DeepSeek {hint}：HTTP {status} {detail}")
}

/// 解析 `/chat/completions` 的非流式响应
pub fn parse_response(text: &str) -> CoreResult<ChatResponse> {
    let resp: CompletionResp = serde_json::from_str(text).map_err(|e| {
        plugin_err(format!("解析 DeepSeek 响应失败：{e}；内容开头：{}", text.chars().take(200).collect::<String>()))
    })?;
    let Some(choice) = resp.choices.into_iter().next() else {
        return Err(plugin_err("DeepSeek 响应里没有 choices"));
    };
    let usage = resp
        .usage
        .map(|u| TokenUsage { prompt_tokens: u.prompt_tokens, completion_tokens: u.completion_tokens, total_tokens: u.total_tokens })
        .unwrap_or_default();
    Ok(ChatResponse {
        content: choice.message.content.unwrap_or_default(),
        model: resp.model,
        usage,
        finish_reason: choice.finish_reason,
    })
}

impl DeepSeekLlmProvider {
    /// API Key 在调用时从 `config.api_key_env` 指定的环境变量读取
    pub fn new(config: DeepSeekConfig) -> CoreResult<Self> {
        Self::build(config, None)
    }

    pub fn with_api_key(config: DeepSeekConfig, api_key: impl Into<String>) -> CoreResult<Self> {
        Self::build(config, Some(api_key.into()))
    }

    fn build(config: DeepSeekConfig, api_key: Option<String>) -> CoreResult<Self> {
        config.validate()?;
        // reqwest 用的是 no-provider 版 rustls：装上进程级默认加密后端（aws-lc-rs）；已经装过时忽略
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs.max(1)))
            .build()
            .map_err(|e| plugin_err(format!("创建 HTTP 客户端失败：{e}")))?;
        Ok(Self { config, api_key, http, backoff_base: Duration::from_millis(500) })
    }

    /// 重试退避的基准间隔（第 n 次重试等待 base × 2^(n-1)，最长 8 秒）
    pub fn with_backoff_base(mut self, base: Duration) -> Self {
        self.backoff_base = base;
        self
    }

    pub fn config(&self) -> &DeepSeekConfig {
        &self.config
    }

    fn url(&self) -> String {
        format!("{}/chat/completions", self.config.base_url.trim_end_matches('/'))
    }

    fn api_key(&self) -> CoreResult<String> {
        self.api_key.clone().filter(|k| !k.trim().is_empty()).or_else(|| self.config.api_key()).ok_or_else(|| {
            plugin_err(format!(
                "没有 DeepSeek API Key：请设置环境变量 {}（获取 Key：https://platform.deepseek.com/api_keys）",
                self.config.api_key_env
            ))
        })
    }

    /// 组装 `/chat/completions` 请求体（单独拿出来方便测试）
    pub fn build_request(&self, req: &ChatRequest) -> CoreResult<Value> {
        if req.messages.is_empty() {
            return Err(CoreError::InvalidParameter("LLM 对话消息为空".into()));
        }
        let model = req.model.as_deref().map(str::trim).filter(|m| !m.is_empty()).unwrap_or(&self.config.model);
        let messages: Vec<Value> = req
            .messages
            .iter()
            .map(|m| {
                let role = match m.role {
                    ChatRole::System => "system",
                    ChatRole::User => "user",
                    ChatRole::Assistant => "assistant",
                };
                json!({ "role": role, "content": m.content })
            })
            .collect();

        let mut body = Map::new();
        body.insert("model".into(), json!(model));
        body.insert("messages".into(), Value::Array(messages));
        body.insert("stream".into(), json!(false));
        if let Some(t) = req.temperature.or(self.config.temperature) {
            check_temperature(t).map_err(CoreError::InvalidParameter)?;
            body.insert("temperature".into(), json!(t));
        }
        if let Some(n) = req.max_tokens.or(self.config.max_tokens) {
            check_max_tokens(n).map_err(CoreError::InvalidParameter)?;
            body.insert("max_tokens".into(), json!(n));
        }
        let format = match req.response_format {
            ResponseFormat::Text => "text",
            ResponseFormat::JsonObject => "json_object",
        };
        body.insert("response_format".into(), json!({ "type": format }));
        if let Some(thinking) = self.config.thinking {
            body.insert("thinking".into(), json!({ "type": if thinking { "enabled" } else { "disabled" } }));
        }
        Ok(Value::Object(body))
    }

    fn backoff(&self, attempt: u32, retry_after: Option<Duration>) -> Duration {
        let exp = self.backoff_base.saturating_mul(1 << (attempt - 1).min(5));
        retry_after.unwrap_or(exp).min(Duration::from_secs(8))
    }

    async fn post(&self, body: &Value) -> CoreResult<String> {
        let api_key = self.api_key()?;
        let mut attempt = 0;
        loop {
            let result = self.http.post(self.url()).bearer_auth(&api_key).json(body).send().await;
            let (err, retry_after) = match result {
                Ok(resp) => {
                    let status = resp.status().as_u16();
                    let retry_after = resp
                        .headers()
                        .get(reqwest::header::RETRY_AFTER)
                        .and_then(|v| v.to_str().ok())
                        .and_then(|v| v.trim().parse::<u64>().ok())
                        .map(Duration::from_secs);
                    let text = resp.text().await.map_err(|e| plugin_err(format!("读取 DeepSeek 响应失败：{e}")))?;
                    if (200..300).contains(&status) {
                        return Ok(text);
                    }
                    let err = http_error_message(status, &text);
                    if !is_retryable(status) {
                        return Err(plugin_err(err));
                    }
                    (err, retry_after)
                }
                Err(e) => (format!("请求 DeepSeek 失败：{e}"), None),
            };
            if attempt >= self.config.max_retries {
                return Err(plugin_err(if attempt > 0 { format!("{err}（已重试 {attempt} 次）") } else { err }));
            }
            attempt += 1;
            let wait = self.backoff(attempt, retry_after);
            tracing::warn!("DeepSeek 请求失败，{} ms 后第 {attempt} 次重试：{err}", wait.as_millis());
            tokio::time::sleep(wait).await;
        }
    }
}

#[async_trait]
impl LlmProvider for DeepSeekLlmProvider {
    fn provider_id(&self) -> &str {
        DRIVER_ID
    }

    async fn chat(&self, req: ChatRequest) -> CoreResult<ChatResponse> {
        let body = self.build_request(&req)?;
        let text = self.post(&body).await?;
        parse_response(&text)
    }
}
