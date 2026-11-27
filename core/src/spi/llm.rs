//! 大语言模型（LLM）扩展点契约。
//!
//! 这里只定义**与协议无关的 canonical 请求 / 响应**。各供应商的 HTTP 细节
//! （OpenAI Chat Completions、OpenAI Responses、Anthropic Messages）放在
//! `leo-llm-protocol` 共享库里做编解码，供应商子插件只负责地址、鉴权和模型参数。
//!
//! canonical 采用 Chat Completions 的语义作为内部标准：消息数组 + 采样参数。
//! 任何协议进来先转 canonical，出去时再转目标协议，这样跨协议转换只有一套逻辑。

use std::pin::Pin;

use async_trait::async_trait;
use futures_core::Stream;
use futures_util::stream;
use serde::{Deserialize, Serialize};

use crate::error::CoreResult;

/// 上游接口协议
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LlmProtocol {
    /// OpenAI 旧接口：`POST /chat/completions`
    #[serde(rename = "openai-chat")]
    OpenAiChat,
    /// OpenAI 新接口：`POST /responses`
    #[serde(rename = "openai-responses")]
    OpenAiResponses,
    /// Anthropic Messages：`POST /v1/messages`
    #[serde(rename = "anthropic")]
    Anthropic,
}

impl LlmProtocol {
    /// 配置 / 日志里使用的字符串
    pub fn as_str(self) -> &'static str {
        match self {
            Self::OpenAiChat => "openai-chat",
            Self::OpenAiResponses => "openai-responses",
            Self::Anthropic => "anthropic",
        }
    }
}

/// 消息角色
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChatRole {
    System,
    User,
    Assistant,
}

/// 一条对话消息
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
    /// 可选说话人名字（OpenAI 的 `name` 字段）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl ChatMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: ChatRole::System,
            content: content.into(),
            name: None,
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: ChatRole::User,
            content: content.into(),
            name: None,
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: ChatRole::Assistant,
            content: content.into(),
            name: None,
        }
    }
}

/// 期望的输出格式。
///
/// - `Text`：普通文本
/// - `JsonObject`：要求模型输出 JSON 对象（OpenAI 的 `json_object`）
/// - `JsonSchema { schema }`：`schema` 是完整的 json_schema 对象，直接交给支持它的协议
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ResponseFormat {
    #[default]
    Text,
    JsonObject,
    JsonSchema {
        schema: serde_json::Value,
    },
}

/// 思考模式开关。
///
/// - `Default`：不显式控制，交给供应商默认行为
/// - `Enabled`：强制开启思考
/// - `Disabled`：关闭思考
/// - `Adaptive`：自适应思考（Anthropic 新模型的 `adaptive`，DeepSeek 按 `enabled` 处理）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ThinkingMode {
    #[default]
    Default,
    Enabled,
    Disabled,
    Adaptive,
}

impl ThinkingMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Enabled => "enabled",
            Self::Disabled => "disabled",
            Self::Adaptive => "adaptive",
        }
    }
}

/// 思考深度（推理强度）。
///
/// 各家的可选档位不同，由供应商插件做映射；canonical 保留完整的公共档位。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReasoningEffort {
    None,
    Minimal,
    Low,
    Medium,
    High,
    XHigh,
    Max,
}

impl ReasoningEffort {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Minimal => "minimal",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::XHigh => "xhigh",
            Self::Max => "max",
        }
    }
}

/// 思考配置（canonical）。
///
/// 供应商配置和单次请求都用这个结构；请求里的非 `Default` 值覆盖插件默认值。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThinkingConfig {
    #[serde(default)]
    pub mode: ThinkingMode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effort: Option<ReasoningEffort>,
    /// Anthropic 手动思考预算；其他供应商可能忽略
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub budget_tokens: Option<u32>,
}

impl ThinkingConfig {
    /// 用 `fallback` 补齐当前配置里没有显式指定的字段。
    pub fn merged_with(&self, fallback: &Self) -> Self {
        Self {
            mode: if self.mode == ThinkingMode::Default {
                fallback.mode
            } else {
                self.mode
            },
            effort: self.effort.or(fallback.effort),
            budget_tokens: self.budget_tokens.or(fallback.budget_tokens),
        }
    }

    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }

    /// 归一化冲突组合：`effort = none` 等价于关闭思考；关闭思考时清掉 effort。
    pub fn normalized(mut self) -> Self {
        if self.effort == Some(ReasoningEffort::None) {
            self.mode = ThinkingMode::Disabled;
            self.effort = None;
        }
        if self.mode == ThinkingMode::Disabled {
            self.effort = None;
            self.budget_tokens = None;
        }
        self
    }
}

/// 对话请求（canonical）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ChatRequest {
    /// 模型 id。走注册层时可能带 `驱动id:` 前缀，路由后这里只剩模型 id。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    pub messages: Vec<ChatMessage>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stop: Vec<String>,
    #[serde(default)]
    pub response_format: ResponseFormat,
    /// `None` 表示使用插件配置的默认值；`Some(true/false)` 显式覆盖。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(default)]
    pub thinking: ThinkingConfig,
}

/// 结束原因（已归一化到各协议的公共语义）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum FinishReason {
    #[default]
    Stop,
    Length,
    ContentFilter,
    ToolCalls,
    Other,
}

/// token 用量（已归一化）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TokenUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub total_tokens: u32,
}

/// 对话响应（canonical）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatResponse {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub model: String,
    pub content: String,
    /// 推理过程（DeepSeek 的 reasoning_content / Anthropic 的 thinking / Responses 的 reasoning）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning_content: Option<String>,
    #[serde(default)]
    pub finish_reason: FinishReason,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage: Option<TokenUsage>,
}

/// 模型信息（列表 / 诊断用）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_window: Option<u32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub capabilities: Vec<String>,
}

/// 流式对话的单个增量。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ChatDelta {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning_content: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<FinishReason>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage: Option<TokenUsage>,
}

impl ChatDelta {
    /// 把一次性响应包装成单个增量（用于不支持流式的驱动的兜底）。
    pub fn from_response(response: &ChatResponse) -> Self {
        Self {
            content: if response.content.is_empty() {
                None
            } else {
                Some(response.content.clone())
            },
            reasoning_content: response.reasoning_content.clone(),
            finish_reason: Some(response.finish_reason),
            usage: response.usage,
        }
    }
}

/// 流式对话输出。每个元素是一个增量，错误以 `Err` 结束。
pub type ChatStream = Pin<Box<dyn Stream<Item = CoreResult<ChatDelta>> + Send>>;

/// 把流式增量拼回完整响应。
#[derive(Debug, Clone)]
pub struct ChatResponseAccumulator {
    model: String,
    content: String,
    reasoning_content: Option<String>,
    finish_reason: FinishReason,
    usage: Option<TokenUsage>,
}

impl ChatResponseAccumulator {
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            content: String::new(),
            reasoning_content: None,
            finish_reason: FinishReason::Stop,
            usage: None,
        }
    }

    pub fn push(&mut self, delta: ChatDelta) {
        if let Some(content) = delta.content {
            self.content.push_str(&content);
        }
        if let Some(reasoning) = delta.reasoning_content {
            self.reasoning_content
                .get_or_insert_with(String::new)
                .push_str(&reasoning);
        }
        if let Some(reason) = delta.finish_reason {
            self.finish_reason = reason;
        }
        if let Some(usage) = delta.usage {
            self.merge_usage(usage);
        }
    }

    fn merge_usage(&mut self, usage: TokenUsage) {
        match &mut self.usage {
            None => self.usage = Some(usage),
            Some(current) => {
                current.input_tokens = current.input_tokens.max(usage.input_tokens);
                current.output_tokens = current.output_tokens.max(usage.output_tokens);
                current.total_tokens = current
                    .total_tokens
                    .max(usage.total_tokens)
                    .max(current.input_tokens + current.output_tokens);
            }
        }
    }

    pub fn finish(self) -> ChatResponse {
        ChatResponse {
            id: None,
            model: self.model,
            content: self.content,
            reasoning_content: self.reasoning_content,
            finish_reason: self.finish_reason,
            usage: self.usage,
        }
    }
}

/// 大语言模型扩展点
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// 驱动 id（如 `deepseek`、`volcengine`、`bailian`）
    fn provider_id(&self) -> &str;

    /// 该驱动配置的上游协议
    fn protocol(&self) -> LlmProtocol;

    /// 该驱动支持的模型列表；拿不到远端列表时允许返回内置兜底清单
    async fn list_models(&self) -> CoreResult<Vec<ModelInfo>>;

    /// 单轮 / 多轮对话。是否走流式由驱动配置决定，返回时已聚合完整。
    async fn chat(&self, request: &ChatRequest) -> CoreResult<ChatResponse>;

    /// 流式对话。默认实现回退到 `chat`，只产出一个增量。
    async fn chat_stream(&self, request: &ChatRequest) -> CoreResult<ChatStream> {
        let response = self.chat(request).await?;
        Ok(Box::pin(stream::once(async move {
            Ok(ChatDelta::from_response(&response))
        })))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn protocol_serde_uses_config_names() {
        assert_eq!(
            serde_json::to_string(&LlmProtocol::OpenAiChat).unwrap(),
            "\"openai-chat\""
        );
        assert_eq!(
            serde_json::to_string(&LlmProtocol::OpenAiResponses).unwrap(),
            "\"openai-responses\""
        );
        assert_eq!(
            serde_json::to_string(&LlmProtocol::Anthropic).unwrap(),
            "\"anthropic\""
        );
        assert_eq!(
            serde_json::from_str::<LlmProtocol>("\"anthropic\"").unwrap(),
            LlmProtocol::Anthropic
        );
    }

    #[test]
    fn request_roundtrip() {
        let req = ChatRequest {
            model: Some("deepseek-chat".into()),
            messages: vec![
                ChatMessage::system("你是校对助手"),
                ChatMessage::user("改错别字"),
            ],
            temperature: Some(0.3),
            top_p: None,
            max_tokens: Some(2048),
            stop: vec!["\n\n".into()],
            response_format: ResponseFormat::JsonObject,
            ..Default::default()
        };
        let value = serde_json::to_value(&req).unwrap();
        assert_eq!(value["messages"][0]["role"], json!("system"));
        assert_eq!(value["response_format"]["type"], json!("json_object"));
        let back: ChatRequest = serde_json::from_value(value).unwrap();
        assert_eq!(back, req);
    }

    #[test]
    fn thinking_none_effort_normalizes_to_disabled() {
        let thinking = ThinkingConfig {
            mode: ThinkingMode::Enabled,
            effort: Some(ReasoningEffort::None),
            budget_tokens: Some(4096),
        }
        .normalized();
        assert_eq!(thinking.mode, ThinkingMode::Disabled);
        assert_eq!(thinking.effort, None);
        assert_eq!(thinking.budget_tokens, None);
    }

    #[test]
    fn thinking_merge_prefers_explicit_values() {
        let request = ThinkingConfig {
            mode: ThinkingMode::Default,
            effort: Some(ReasoningEffort::Low),
            budget_tokens: None,
        };
        let fallback = ThinkingConfig {
            mode: ThinkingMode::Enabled,
            effort: Some(ReasoningEffort::High),
            budget_tokens: Some(8192),
        };
        let merged = request.merged_with(&fallback);
        assert_eq!(merged.mode, ThinkingMode::Enabled);
        assert_eq!(merged.effort, Some(ReasoningEffort::Low));
        assert_eq!(merged.budget_tokens, Some(8192));
    }
}
