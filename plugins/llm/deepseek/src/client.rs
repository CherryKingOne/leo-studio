//! DeepSeek HTTP 接口调用（两种接口风格，Bearer 鉴权，非流式，共用重试与错误映射）。
//!
//! - Chat Completions：`POST {base_url}/chat/completions`
//!   <https://api-docs.deepseek.com/api/create-chat-completion>
//!   输出格式 `response_format.type` 只有 `text` / `json_object`（官方文档没有 `json_schema`）
//!   <https://api-docs.deepseek.com/guides/json_mode>
//! - Responses：`POST {base_url}/responses`（OpenAI Responses 格式，无状态）
//!   <https://api-docs.deepseek.com/api/create-response>、<https://api-docs.deepseek.com/guides/responses_api>
//!   `input` 为消息列表；`text.format` 支持 `text` / `json_object` / `json_schema`（name + schema）；
//!   `max_output_tokens`；思考模式用 `reasoning.effort`（none 关闭 / low / high / max）；
//!   结果在 `output[]` 的 `message` 项的 `output_text` 里，`status` 为 completed / incomplete / failed
//! - 错误码：<https://api-docs.deepseek.com/quick_start/error_codes>（429 / 5xx 可重试）

use std::time::Duration;

use async_trait::async_trait;
use leo_core::error::{CoreError, CoreResult};
use leo_core::spi::{
    ChatRequest, ChatResponse, ChatRole, LlmApi, LlmApiCapabilities, LlmCapabilities, LlmProvider, ResponseFormat, TokenUsage,
};
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

// ---------- Responses 接口的响应结构 ----------

#[derive(Deserialize)]
struct ResponsesResp {
    #[serde(default)]
    model: String,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    error: Option<ResponsesError>,
    #[serde(default)]
    incomplete_details: Option<IncompleteDetails>,
    #[serde(default)]
    output: Vec<OutputItem>,
    #[serde(default)]
    usage: Option<ResponsesUsage>,
}

#[derive(Deserialize)]
struct ResponsesError {
    #[serde(default)]
    code: Option<String>,
    #[serde(default)]
    message: Option<String>,
}

#[derive(Deserialize)]
struct IncompleteDetails {
    #[serde(default)]
    reason: Option<String>,
}

#[derive(Deserialize)]
struct OutputItem {
    #[serde(default, rename = "type")]
    kind: String,
    #[serde(default)]
    content: Vec<OutputContent>,
}

#[derive(Deserialize)]
struct OutputContent {
    #[serde(default, rename = "type")]
    kind: String,
    #[serde(default)]
    text: Option<String>,
}

#[derive(Deserialize)]
struct ResponsesUsage {
    #[serde(default)]
    input_tokens: u32,
    #[serde(default)]
    output_tokens: u32,
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

/// 解析 `/responses` 的非流式响应，映射到统一的 [`ChatResponse`]：
/// - `content` = 所有 `message` 项里 `output_text` 的拼接（`reasoning` 思维链项忽略）
/// - `usage`：input_tokens → prompt_tokens，output_tokens → completion_tokens，total_tokens 原样
/// - `finish_reason`：completed → `stop`；incomplete 且 reason=max_output_tokens → `length`，
///   reason=content_filter → `content_filter`；failed → 返回错误（带 error.code / message）
pub fn parse_responses_response(text: &str) -> CoreResult<ChatResponse> {
    let resp: ResponsesResp = serde_json::from_str(text).map_err(|e| {
        plugin_err(format!(
            "解析 DeepSeek Responses 响应失败：{e}；内容开头：{}",
            text.chars().take(200).collect::<String>()
        ))
    })?;
    let finish_reason = match resp.status.as_deref() {
        Some("completed") => Some("stop".to_string()),
        Some("incomplete") => Some(
            match resp.incomplete_details.and_then(|d| d.reason).as_deref() {
                Some("max_output_tokens") => "length",
                Some("content_filter") => "content_filter",
                _ => "incomplete",
            }
            .to_string(),
        ),
        Some("failed") => {
            let (code, message) =
                resp.error.map(|e| (e.code.unwrap_or_default(), e.message.unwrap_or_default())).unwrap_or_default();
            return Err(plugin_err(format!("DeepSeek Responses 生成失败（status=failed）：{code} {message}").trim_end().to_string()));
        }
        Some(other) => return Err(plugin_err(format!("DeepSeek Responses 返回了未完成的状态 {other}（非流式请求不应出现）"))),
        None => None,
    };
    let messages: Vec<&OutputItem> = resp.output.iter().filter(|o| o.kind == "message").collect();
    if messages.is_empty() && finish_reason.as_deref() == Some("stop") {
        return Err(plugin_err("DeepSeek Responses 响应的 output 里没有 message 项"));
    }
    let content: String = messages
        .iter()
        .flat_map(|m| m.content.iter())
        .filter(|c| c.kind == "output_text")
        .filter_map(|c| c.text.as_deref())
        .collect();
    let usage = resp
        .usage
        .map(|u| TokenUsage { prompt_tokens: u.input_tokens, completion_tokens: u.output_tokens, total_tokens: u.total_tokens })
        .unwrap_or_default();
    Ok(ChatResponse { content, model: resp.model, usage, finish_reason })
}

fn role_str(role: ChatRole) -> &'static str {
    match role {
        ChatRole::System => "system",
        ChatRole::User => "user",
        ChatRole::Assistant => "assistant",
    }
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

    fn url(&self, api: LlmApi) -> String {
        let path = match api {
            LlmApi::ChatCompletions => "chat/completions",
            LlmApi::Responses => "responses",
        };
        format!("{}/{path}", self.config.base_url.trim_end_matches('/'))
    }

    /// 本次请求实际使用的接口（请求里的 `api` 优先，否则配置的 `api`）
    pub fn effective_api(&self, req: &ChatRequest) -> LlmApi {
        req.api.unwrap_or(self.config.api)
    }

    fn model<'a>(&'a self, req: &'a ChatRequest) -> &'a str {
        req.model.as_deref().map(str::trim).filter(|m| !m.is_empty()).unwrap_or(&self.config.model)
    }

    fn api_key(&self) -> CoreResult<String> {
        self.api_key.clone().filter(|k| !k.trim().is_empty()).or_else(|| self.config.api_key()).ok_or_else(|| {
            plugin_err(format!(
                "没有 DeepSeek API Key：请设置环境变量 {}（获取 Key：https://platform.deepseek.com/api_keys）",
                self.config.api_key_env
            ))
        })
    }

    /// 按本次请求的接口组装请求体，返回 (接口, 请求体)
    pub fn build_body(&self, req: &ChatRequest) -> CoreResult<(LlmApi, Value)> {
        let api = self.effective_api(req);
        let body = match api {
            LlmApi::ChatCompletions => self.build_request(req)?,
            LlmApi::Responses => self.build_responses_request(req)?,
        };
        Ok((api, body))
    }

    /// 组装 `/chat/completions` 请求体（单独拿出来方便测试）
    pub fn build_request(&self, req: &ChatRequest) -> CoreResult<Value> {
        if req.messages.is_empty() {
            return Err(CoreError::InvalidParameter("LLM 对话消息为空".into()));
        }
        let format = match &req.response_format {
            ResponseFormat::Text => "text",
            ResponseFormat::JsonObject => "json_object",
            ResponseFormat::JsonSchema { .. } => {
                return Err(CoreError::InvalidParameter(
                    "DeepSeek Chat Completions 接口不支持 json_schema（官方 response_format 只有 text / json_object）。\
                     请改用 api=responses，或经 LlmHub 调用并设置 llm.schema_fallback=json_object"
                        .into(),
                ));
            }
        };
        let messages: Vec<Value> =
            req.messages.iter().map(|m| json!({ "role": role_str(m.role), "content": m.content })).collect();

        let mut body = Map::new();
        body.insert("model".into(), json!(self.model(req)));
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
        body.insert("response_format".into(), json!({ "type": format }));
        if let Some(thinking) = self.config.thinking {
            body.insert("thinking".into(), json!({ "type": if thinking { "enabled" } else { "disabled" } }));
        }
        if let Some(effort) = self.config.reasoning_effort {
            body.insert("reasoning_effort".into(), json!(effort.as_str()));
        }
        Ok(Value::Object(body))
    }

    /// 组装 `/responses` 请求体（单独拿出来方便测试）
    ///
    /// - 消息按顺序放进 `input`（`{"type": "message", "role", "content"}`，system 角色官方支持）
    /// - `max_tokens` → `max_output_tokens`
    /// - 输出格式 → `text.format`：`{"type":"text"}` / `{"type":"json_object"}` /
    ///   `{"type":"json_schema","name","schema","strict"}`
    /// - 思考模式 → `reasoning.effort`：thinking=false → `none`；否则 reasoning_effort（不填不传）
    pub fn build_responses_request(&self, req: &ChatRequest) -> CoreResult<Value> {
        if req.messages.is_empty() {
            return Err(CoreError::InvalidParameter("LLM 对话消息为空".into()));
        }
        let format = match &req.response_format {
            ResponseFormat::Text => json!({ "type": "text" }),
            ResponseFormat::JsonObject => json!({ "type": "json_object" }),
            ResponseFormat::JsonSchema { name, schema, strict } => {
                if name.trim().is_empty() {
                    return Err(CoreError::InvalidParameter("json_schema 的 name 不能为空（DeepSeek Responses 要求必填）".into()));
                }
                if !schema.is_object() {
                    return Err(CoreError::InvalidParameter(format!("json_schema 的 schema 必须是 JSON 对象，收到 {schema}")));
                }
                json!({ "type": "json_schema", "name": name, "schema": schema, "strict": strict })
            }
        };
        let input: Vec<Value> = req
            .messages
            .iter()
            .map(|m| json!({ "type": "message", "role": role_str(m.role), "content": m.content }))
            .collect();

        let mut body = Map::new();
        body.insert("model".into(), json!(self.model(req)));
        body.insert("input".into(), Value::Array(input));
        body.insert("stream".into(), json!(false));
        if let Some(t) = req.temperature.or(self.config.temperature) {
            check_temperature(t).map_err(CoreError::InvalidParameter)?;
            body.insert("temperature".into(), json!(t));
        }
        if let Some(n) = req.max_tokens.or(self.config.max_tokens) {
            check_max_tokens(n).map_err(CoreError::InvalidParameter)?;
            body.insert("max_output_tokens".into(), json!(n));
        }
        body.insert("text".into(), json!({ "format": format }));
        let effort = match (self.config.thinking, self.config.reasoning_effort) {
            (Some(false), _) => Some("none"),
            (_, Some(e)) => Some(e.as_str()),
            _ => None,
        };
        if let Some(effort) = effort {
            body.insert("reasoning".into(), json!({ "effort": effort }));
        }
        Ok(Value::Object(body))
    }

    fn backoff(&self, attempt: u32, retry_after: Option<Duration>) -> Duration {
        let exp = self.backoff_base.saturating_mul(1 << (attempt - 1).min(5));
        retry_after.unwrap_or(exp).min(Duration::from_secs(8))
    }

    async fn post(&self, api: LlmApi, body: &Value) -> CoreResult<String> {
        let api_key = self.api_key()?;
        let url = self.url(api);
        let mut attempt = 0;
        loop {
            let result = self.http.post(&url).bearer_auth(&api_key).json(body).send().await;
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
            tracing::warn!("DeepSeek {api} 请求失败，{} ms 后第 {attempt} 次重试：{err}", wait.as_millis());
            tokio::time::sleep(wait).await;
        }
    }
}

#[async_trait]
impl LlmProvider for DeepSeekLlmProvider {
    fn provider_id(&self) -> &str {
        DRIVER_ID
    }

    /// 两种接口都支持；JsonSchema 只有 Responses 原生支持（官方 Chat Completions 的 response_format 没有 json_schema）
    fn capabilities(&self) -> LlmCapabilities {
        LlmCapabilities {
            default_api: self.config.api,
            apis: vec![
                LlmApiCapabilities { api: LlmApi::ChatCompletions, json_object: true, json_schema: false },
                LlmApiCapabilities { api: LlmApi::Responses, json_object: true, json_schema: true },
            ],
        }
    }

    async fn chat(&self, req: ChatRequest) -> CoreResult<ChatResponse> {
        let (api, body) = self.build_body(&req)?;
        let text = self.post(api, &body).await?;
        match api {
            LlmApi::ChatCompletions => parse_response(&text),
            LlmApi::Responses => parse_responses_response(&text),
        }
    }
}
