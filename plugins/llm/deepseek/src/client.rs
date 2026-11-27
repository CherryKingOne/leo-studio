//! DeepSeek HTTP 调用。
//!
//! 三种协议共用同一套流程：
//! 1. 合并配置里的默认采样参数；
//! 2. 交给 `leo-llm-protocol` 编码成对应协议的 JSON；
//! 3. 按协议拼地址 / 鉴权头（Anthropic 用 `x-api-key` + `anthropic-version`）；
//! 4. 合并 `extra_body` / `extra_headers`；
//! 5. 解码成 canonical，错误体统一映射。

use std::collections::VecDeque;
use std::time::Duration;

use async_trait::async_trait;
use futures_util::StreamExt;
use leo_core::error::{CoreError, CoreResult};
use leo_core::spi::{
    ChatRequest, ChatResponse, ChatResponseAccumulator, ChatStream, LlmProtocol, LlmProvider,
    ModelInfo, ReasoningEffort, ThinkingMode,
};
use serde_json::{Value, json};

use crate::PLUGIN_ID;
use crate::config::DeepseekConfig;
use crate::models::builtin_models;

pub struct DeepseekLlmProvider {
    config: DeepseekConfig,
    api_key: String,
    http: reqwest::Client,
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

/// DeepSeek 只认 `high` / `max`；其余档位就近映射。
fn deepseek_effort(effort: ReasoningEffort) -> &'static str {
    match effort {
        ReasoningEffort::None => "none",
        ReasoningEffort::Minimal
        | ReasoningEffort::Low
        | ReasoningEffort::Medium
        | ReasoningEffort::High => "high",
        ReasoningEffort::XHigh | ReasoningEffort::Max => "max",
    }
}

impl DeepseekLlmProvider {
    pub fn new(config: DeepseekConfig, api_key: String) -> CoreResult<Self> {
        config.validate()?;
        // reqwest 用的是 no-provider 版 rustls：这里装上进程级默认加密后端（aws-lc-rs，
        // 与 msedge-tts 一致）。已经装过时 install_default 返回 Err，忽略即可。
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
        })
    }

    pub fn config(&self) -> &DeepseekConfig {
        &self.config
    }

    fn url(&self) -> String {
        leo_llm_protocol::endpoint(&self.config.base_url, self.config.protocol)
    }

    /// 组装请求体（单独拿出来方便测试）
    pub fn build_body(&self, request: &ChatRequest) -> CoreResult<Value> {
        let model = leo_llm_protocol::resolve_model(request.model.as_deref(), &self.config.model)?;
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
        let mut body = leo_llm_protocol::encode_request(self.config.protocol, &request, model);
        if let Value::Object(map) = &mut body {
            for (key, value) in &self.config.extra_body {
                map.insert(key.clone(), value.clone());
            }
            // DeepSeek 的思考开关不在 OpenAI 标准字段里，必须走 thinking.type。
            if self.config.protocol == LlmProtocol::OpenAiChat
                && request.thinking.mode != ThinkingMode::Default
            {
                let thinking_type = match request.thinking.mode {
                    ThinkingMode::Enabled | ThinkingMode::Adaptive => "enabled",
                    ThinkingMode::Disabled => "disabled",
                    ThinkingMode::Default => "enabled",
                };
                map.insert(
                    "thinking".into(),
                    serde_json::json!({ "type": thinking_type }),
                );
            }
            // 协议库按 OpenAI 语义写档位，DeepSeek 的档位只有 high / max。
            if let Some(effort) = request
                .thinking
                .effort
                .filter(|_| request.thinking.mode != ThinkingMode::Disabled)
            {
                match self.config.protocol {
                    LlmProtocol::OpenAiChat => {
                        map.insert("reasoning_effort".into(), json!(deepseek_effort(effort)));
                    }
                    LlmProtocol::OpenAiResponses => {
                        if let Some(reasoning) =
                            map.get_mut("reasoning").and_then(Value::as_object_mut)
                        {
                            reasoning.insert("effort".into(), json!(deepseek_effort(effort)));
                        }
                    }
                    LlmProtocol::Anthropic => {}
                }
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
            .map_err(|error| plugin_err(format!("请求 DeepSeek 失败：{error}")))?;
        let status = response.status();
        let text = response
            .text()
            .await
            .map_err(|error| plugin_err(format!("读取 DeepSeek 响应失败：{error}")))?;
        if !status.is_success() {
            return Err(leo_llm_protocol::decode_error(
                PLUGIN_ID,
                self.config.protocol,
                status.as_u16(),
                &text,
            ));
        }
        let value: Value = serde_json::from_str(&text)
            .map_err(|error| plugin_err(format!("DeepSeek 响应不是合法 JSON：{error}")))?;
        leo_llm_protocol::decode_response(self.config.protocol, &value)
    }

    async fn request_stream(&self, request: &ChatRequest) -> CoreResult<ChatStream> {
        let mut streaming = request.clone();
        streaming.stream = Some(true);
        let body = self.build_body(&streaming)?;
        let response = self
            .request_builder(&body)
            .send()
            .await
            .map_err(|error| plugin_err(format!("请求 DeepSeek 流式接口失败：{error}")))?;
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
                                Err(plugin_err(format!("读取 DeepSeek 流式响应失败：{error}"))),
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
}

#[async_trait]
impl LlmProvider for DeepseekLlmProvider {
    fn provider_id(&self) -> &str {
        crate::DRIVER_ID
    }

    fn protocol(&self) -> LlmProtocol {
        self.config.protocol
    }

    async fn list_models(&self) -> CoreResult<Vec<ModelInfo>> {
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
    use super::*;
    use leo_core::spi::{ChatMessage, ThinkingConfig};

    #[test]
    fn body_uses_request_model_over_config_and_merges_extra_body() {
        let config = DeepseekConfig {
            extra_body: [(
                "thinking".to_string(),
                serde_json::json!({"type": "enabled"}),
            )]
            .into_iter()
            .collect(),
            ..Default::default()
        };
        let provider = DeepseekLlmProvider::new(config, "k".into()).unwrap();
        let request = ChatRequest {
            model: Some("deepseek-reasoner".into()),
            messages: vec![ChatMessage::user("你好")],
            ..Default::default()
        };
        let body = provider.build_body(&request).unwrap();
        assert_eq!(body["model"], "deepseek-reasoner");
        assert_eq!(body["messages"][0]["content"], "你好");
        assert_eq!(body["thinking"]["type"], "enabled");
    }

    #[test]
    fn empty_model_and_config_errors() {
        let provider = DeepseekLlmProvider::new(
            DeepseekConfig {
                model: String::new(),
                ..Default::default()
            },
            "k".into(),
        )
        .unwrap();
        let request = ChatRequest {
            messages: vec![ChatMessage::user("x")],
            ..Default::default()
        };
        assert!(provider.build_body(&request).is_err());
    }

    #[test]
    fn reasoning_effort_maps_to_deepseek_levels() {
        let provider = DeepseekLlmProvider::new(DeepseekConfig::default(), "k".into()).unwrap();
        for (effort, expected) in [
            (ReasoningEffort::Low, "high"),
            (ReasoningEffort::Medium, "high"),
            (ReasoningEffort::High, "high"),
            (ReasoningEffort::XHigh, "max"),
            (ReasoningEffort::Max, "max"),
        ] {
            let body = provider
                .build_body(&ChatRequest {
                    messages: vec![ChatMessage::user("x")],
                    thinking: ThinkingConfig {
                        mode: ThinkingMode::Enabled,
                        effort: Some(effort),
                        budget_tokens: None,
                    },
                    ..Default::default()
                })
                .unwrap();
            assert_eq!(body["reasoning_effort"], json!(expected));
            assert_eq!(body["thinking"]["type"], json!("enabled"));
        }
    }
}
