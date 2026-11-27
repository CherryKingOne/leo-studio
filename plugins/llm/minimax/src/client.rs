//! 统一 HTTP 调用层。
//!
//! 三种协议共用同一套流程：
//! 1. 合并配置里的默认采样参数；
//! 2. 交给 `leo-llm-protocol` 编码成对应协议的 JSON；
//! 3. 按协议拼地址 / 鉴权头（Anthropic 用 `x-api-key` + `anthropic-version`）；
//! 4. 合并 `extra_body` / `extra_headers`；
//! 5. 解码成 canonical，错误体统一映射。
//!
//! 模型列表：配置了 `models_url` 就远端拉取并过滤“自家模型”，失败时回退
//! `models.rs` 的内置清单；没有配置的厂商直接返回内置清单。

use std::collections::VecDeque;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use futures_util::StreamExt;
use leo_core::error::{CoreError, CoreResult};
use leo_core::spi::{
    ChatRequest, ChatResponse, ChatResponseAccumulator, ChatStream, LlmProtocol, LlmProvider,
    ModelInfo, ThinkingMode,
};
use serde_json::{Value, json};

use crate::config::Config;
use crate::models::{builtin_models, ensure_default, is_owned_model};
use crate::{DISPLAY_NAME, DRIVER_ID, INSERTS_THINKING_OBJECT, PLUGIN_ID};

pub struct Provider {
    config: Config,
    api_key: String,
    http: reqwest::Client,
    models_cache: Mutex<Option<(Instant, Vec<ModelInfo>)>>,
}

fn plugin_err(message: impl Into<String>) -> CoreError {
    CoreError::Plugin {
        plugin_id: PLUGIN_ID.into(),
        message: message.into(),
    }
}

fn is_loopback(base_url: &str) -> bool {
    let url = base_url.trim();
    url.starts_with("http://127.0.0.1")
        || url.starts_with("http://localhost")
        || url.starts_with("http://[::1]")
}

impl Provider {
    pub fn new(config: Config, api_key: String) -> CoreResult<Self> {
        config.validate()?;
        if api_key.trim().is_empty() {
            return Err(plugin_err("API Key 为空"));
        }
        // reqwest 用的是 no-provider 版 rustls：这里装上进程级默认加密后端。
        // 已经装过时 install_default 返回 Err，忽略即可。
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
        let mut builder =
            reqwest::Client::builder().timeout(Duration::from_secs(config.timeout_secs.max(1)));
        // 本地假服务器 / 本地网关不该走 HTTP(S)_PROXY。
        if is_loopback(&config.base_url) {
            builder = builder.no_proxy();
        }
        let http = builder
            .build()
            .map_err(|error| plugin_err(format!("创建 HTTP 客户端失败：{error}")))?;
        Ok(Self {
            config,
            api_key,
            http,
            models_cache: Mutex::new(None),
        })
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    fn url(&self) -> String {
        leo_llm_protocol::endpoint(&self.config.base_url, self.config.protocol)
    }

    /// 组装请求体（单独拿出来方便测试）
    pub fn build_body(&self, request: &ChatRequest) -> CoreResult<Value> {
        let model = leo_llm_protocol::resolve_model(request.model.as_deref(), &self.config.model)?
            .to_string();
        if !is_owned_model(&model, &self.config.allowed_models) {
            return Err(plugin_err(format!(
                "模型 `{model}` 不属于 {DISPLAY_NAME} 的自有模型范围；可用模型见模型列表，或把自定义模型写进 allowed_models"
            )));
        }

        let mut request = request.clone();
        if request.temperature.is_none() {
            request.temperature = self.config.temperature;
        }
        if request.top_p.is_none() {
            request.top_p = self.config.top_p;
        }
        if request.max_tokens.is_none() {
            request.max_tokens = self.config.max_tokens;
        }
        request.stream = Some(request.stream.unwrap_or(self.config.stream));
        request.thinking = request
            .thinking
            .merged_with(&self.config.thinking)
            .normalized();

        let mut body = leo_llm_protocol::encode_request(self.config.protocol, &request, &model);
        if let Value::Object(map) = &mut body {
            for (key, value) in &self.config.extra_body {
                map.insert(key.clone(), value.clone());
            }
            // 智谱的思考开关不在 OpenAI 标准字段里，必须在 chat 请求体里写 thinking.type。
            if INSERTS_THINKING_OBJECT
                && self.config.protocol == LlmProtocol::OpenAiChat
                && request.thinking.mode != ThinkingMode::Default
            {
                let thinking_type = match request.thinking.mode {
                    ThinkingMode::Enabled | ThinkingMode::Adaptive => "enabled",
                    ThinkingMode::Disabled => "disabled",
                    ThinkingMode::Default => "enabled",
                };
                map.insert("thinking".into(), json!({ "type": thinking_type }));
            }
        }
        Ok(body)
    }

    fn request_builder(&self, body: &Value) -> reqwest::RequestBuilder {
        let mut builder = self.http.post(self.url()).json(body);
        for (name, value) in leo_llm_protocol::auth_headers(self.config.protocol, &self.api_key) {
            builder = builder.header(name, value);
        }
        for (name, value) in &self.config.extra_headers {
            builder = builder.header(name, value);
        }
        builder
    }

    async fn request(&self, request: &ChatRequest) -> CoreResult<ChatResponse> {
        let body = self.build_body(request)?;
        let response = self
            .request_builder(&body)
            .send()
            .await
            .map_err(|error| plugin_err(format!("请求 {DISPLAY_NAME} 失败：{error}")))?;
        let status = response.status();
        let text = response
            .text()
            .await
            .map_err(|error| plugin_err(format!("读取 {DISPLAY_NAME} 响应失败：{error}")))?;
        if !status.is_success() {
            return Err(leo_llm_protocol::decode_error(
                PLUGIN_ID,
                self.config.protocol,
                status.as_u16(),
                &text,
            ));
        }
        let value: Value = serde_json::from_str(&text)
            .map_err(|error| plugin_err(format!("{DISPLAY_NAME} 响应不是合法 JSON：{error}")))?;
        leo_llm_protocol::decode_response(self.config.protocol, &value)
    }

    async fn request_stream(&self, request: &ChatRequest) -> CoreResult<ChatStream> {
        let mut streaming = request.clone();
        streaming.stream = Some(true);
        let body = self.build_body(&streaming)?;
        let response =
            self.request_builder(&body).send().await.map_err(|error| {
                plugin_err(format!("请求 {DISPLAY_NAME} 流式接口失败：{error}"))
            })?;
        let status = response.status();
        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(leo_llm_protocol::decode_error(
                PLUGIN_ID,
                self.config.protocol,
                status.as_u16(),
                &text,
            ));
        }

        let protocol = self.config.protocol;
        let bytes = response.bytes_stream();
        let parser = leo_llm_protocol::SseParser::new();
        let pending: VecDeque<String> = VecDeque::new();
        let done = false;
        let stream = futures_util::stream::unfold(
            (bytes, parser, pending, done),
            move |(mut bytes, mut parser, mut pending, mut done)| async move {
                loop {
                    if let Some(payload) = pending.pop_front() {
                        if payload.trim() == "[DONE]" {
                            return None;
                        }
                        match leo_llm_protocol::decode_stream_event(protocol, &payload) {
                            Ok(Some(delta)) => {
                                return Some((Ok(delta), (bytes, parser, pending, done)));
                            }
                            Ok(None) => continue,
                            Err(error) => {
                                return Some((Err(error), (bytes, parser, pending, true)));
                            }
                        }
                    }
                    if done {
                        return None;
                    }
                    match bytes.next().await {
                        Some(Ok(chunk)) => {
                            pending.extend(parser.feed(&chunk));
                        }
                        Some(Err(error)) => {
                            return Some((
                                Err(plugin_err(format!(
                                    "读取 {DISPLAY_NAME} 流式响应失败：{error}"
                                ))),
                                (bytes, parser, pending, true),
                            ));
                        }
                        None => {
                            done = true;
                            pending.extend(parser.finish());
                            if pending.is_empty() {
                                return None;
                            }
                        }
                    }
                }
            },
        );
        Ok(Box::pin(stream))
    }

    async fn fetch_remote_models(&self, url: &str) -> CoreResult<Vec<ModelInfo>> {
        if self.config.models_cache_secs > 0
            && let Ok(cache) = self.models_cache.lock()
            && let Some((fetched_at, models)) = cache.as_ref()
            && fetched_at.elapsed() < Duration::from_secs(self.config.models_cache_secs)
        {
            return Ok(models.clone());
        }

        let response = self
            .http
            .get(url)
            .bearer_auth(&self.api_key)
            .send()
            .await
            .map_err(|error| plugin_err(format!("请求 {DISPLAY_NAME} 模型列表失败：{error}")))?;
        let status = response.status();
        let text = response
            .text()
            .await
            .map_err(|error| plugin_err(format!("读取 {DISPLAY_NAME} 模型列表失败：{error}")))?;
        if !status.is_success() {
            return Err(plugin_err(format!(
                "{DISPLAY_NAME} 模型列表返回 HTTP {}：{}",
                status.as_u16(),
                text.chars().take(300).collect::<String>()
            )));
        }
        let value: Value = serde_json::from_str(&text).map_err(|error| {
            plugin_err(format!("{DISPLAY_NAME} 模型列表不是合法 JSON：{error}"))
        })?;
        let data = value
            .get("data")
            .and_then(Value::as_array)
            .or_else(|| value.get("models").and_then(Value::as_array))
            .ok_or_else(|| plugin_err(format!("{DISPLAY_NAME} 模型列表缺少 data/models 数组")))?;

        let mut models = Vec::new();
        for item in data {
            let Some(raw_id) = item
                .get("id")
                .and_then(Value::as_str)
                .or_else(|| item.get("name").and_then(Value::as_str))
            else {
                continue;
            };
            // Gemini 的 OpenAI 兼容列表会返回 `models/gemini-...`。
            let id = raw_id
                .rsplit('/')
                .next()
                .unwrap_or(raw_id)
                .trim()
                .to_string();
            if id.is_empty()
                || !is_owned_model(&id, &self.config.allowed_models)
                || models.iter().any(|model: &ModelInfo| model.id == id)
            {
                continue;
            }
            let name = item
                .get("name")
                .and_then(Value::as_str)
                .map(|name| name.rsplit('/').next().unwrap_or(name))
                .unwrap_or(&id)
                .to_string();
            let context_window = item
                .get("context_window")
                .and_then(Value::as_u64)
                .or_else(|| item.get("max_context_length").and_then(Value::as_u64))
                .map(|value| value as u32);
            models.push(ModelInfo {
                id,
                name,
                context_window,
                capabilities: vec!["chat".into()],
            });
        }

        if !models.is_empty() {
            ensure_default(&mut models, &self.config.model);
            if self.config.models_cache_secs > 0
                && let Ok(mut cache) = self.models_cache.lock()
            {
                *cache = Some((Instant::now(), models.clone()));
            }
        }
        Ok(models)
    }
}

#[async_trait]
impl LlmProvider for Provider {
    fn provider_id(&self) -> &str {
        DRIVER_ID
    }

    fn protocol(&self) -> LlmProtocol {
        self.config.protocol
    }

    async fn list_models(&self) -> CoreResult<Vec<ModelInfo>> {
        if let Some(url) = self.config.models_url.as_deref() {
            match self.fetch_remote_models(url).await {
                Ok(models) if !models.is_empty() => return Ok(models),
                Ok(_) => tracing::warn!("{DISPLAY_NAME} 远端模型列表为空，使用内置清单"),
                Err(error) => tracing::warn!("{DISPLAY_NAME} 获取模型列表失败：{error}"),
            }
        }
        Ok(builtin_models(&self.config.model))
    }

    async fn chat(&self, request: &ChatRequest) -> CoreResult<ChatResponse> {
        let model = leo_llm_protocol::resolve_model(request.model.as_deref(), &self.config.model)?
            .to_string();
        if request.stream.unwrap_or(self.config.stream) {
            let mut stream = self.request_stream(request).await?;
            let mut accumulator = ChatResponseAccumulator::new(model);
            while let Some(item) = stream.next().await {
                accumulator.push(item?);
            }
            Ok(accumulator.finish())
        } else {
            self.request(request).await
        }
    }

    async fn chat_stream(&self, request: &ChatRequest) -> CoreResult<ChatStream> {
        self.request_stream(request).await
    }
}

#[cfg(test)]
mod tests {
    use leo_core::spi::ChatMessage;

    use super::*;

    #[test]
    fn body_uses_request_model_and_merges_extra_body() {
        let config = Config {
            extra_body: [("custom".to_string(), json!(true))].into_iter().collect(),
            stream: false,
            ..Default::default()
        };
        let provider = Provider::new(config, "k".into()).unwrap();
        let request = ChatRequest {
            messages: vec![ChatMessage::user("你好")],
            ..Default::default()
        };
        let body = provider.build_body(&request).unwrap();
        assert_eq!(body["model"], json!(crate::DEFAULT_MODEL));
        assert_eq!(body["messages"][0]["content"], json!("你好"));
        assert_eq!(body["custom"], json!(true));
    }

    #[test]
    fn foreign_model_is_rejected() {
        let provider = Provider::new(Config::default(), "k".into()).unwrap();
        let request = ChatRequest {
            model: Some("deepseek-chat".into()),
            messages: vec![ChatMessage::user("x")],
            ..Default::default()
        };
        assert!(provider.build_body(&request).is_err());
    }
}
