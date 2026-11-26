//! 大语言模型（LLM）扩展点契约。
//!
//! 这里只定义**与协议无关的 canonical 请求 / 响应**。各供应商的 HTTP 细节
//! （OpenAI Chat Completions、OpenAI Responses、Anthropic Messages）放在
//! `leo-llm-protocol` 共享库里做编解码，供应商子插件只负责地址、鉴权和模型参数。
//!
//! canonical 采用 Chat Completions 的语义作为内部标准：消息数组 + 采样参数。
//! 任何协议进来先转 canonical，出去时再转目标协议，这样跨协议转换只有一套逻辑。

use async_trait::async_trait;
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

/// 大语言模型扩展点
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// 驱动 id（如 `deepseek`、`volcengine`、`bailian`）
    fn provider_id(&self) -> &str;

    /// 该驱动配置的上游协议
    fn protocol(&self) -> LlmProtocol;

    /// 该驱动支持的模型列表；拿不到远端列表时允许返回内置兜底清单
    async fn list_models(&self) -> CoreResult<Vec<ModelInfo>>;

    /// 单轮 / 多轮对话（非流式）
    async fn chat(&self, request: &ChatRequest) -> CoreResult<ChatResponse>;
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
        };
        let value = serde_json::to_value(&req).unwrap();
        assert_eq!(value["messages"][0]["role"], json!("system"));
        assert_eq!(value["response_format"]["type"], json!("json_object"));
        let back: ChatRequest = serde_json::from_value(value).unwrap();
        assert_eq!(back, req);
    }
}
