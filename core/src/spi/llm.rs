use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::{CoreError, CoreResult};

/// 对话消息角色
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
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
}

impl ChatMessage {
    pub fn new(role: ChatRole, content: impl Into<String>) -> Self {
        Self { role, content: content.into() }
    }

    pub fn system(content: impl Into<String>) -> Self {
        Self::new(ChatRole::System, content)
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self::new(ChatRole::User, content)
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self::new(ChatRole::Assistant, content)
    }
}

/// 输出格式
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResponseFormat {
    #[default]
    Text,
    /// 要求模型输出合法 JSON 对象（提示词里需要说明要 JSON）
    JsonObject,
}

/// 对话请求；`None` 的字段交给驱动用自己的配置
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ChatRequest {
    /// 覆盖驱动配置的模型
    pub model: Option<String>,
    pub messages: Vec<ChatMessage>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    #[serde(default)]
    pub response_format: ResponseFormat,
}

impl ChatRequest {
    pub fn new(messages: Vec<ChatMessage>) -> Self {
        Self { messages, ..Default::default() }
    }

    pub fn json(mut self) -> Self {
        self.response_format = ResponseFormat::JsonObject;
        self
    }
}

/// Token 用量
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// 对话结果
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatResponse {
    pub content: String,
    /// 实际使用的模型
    pub model: String,
    pub usage: TokenUsage,
    /// 结束原因，如 stop / length / content_filter
    pub finish_reason: Option<String>,
}

impl ChatResponse {
    /// 把 `content` 解析为 JSON；内容为空或不是合法 JSON 时报错
    pub fn json(&self) -> CoreResult<serde_json::Value> {
        parse_json_content(&self.content, self.finish_reason.as_deref())
    }
}

fn parse_json_content(content: &str, finish_reason: Option<&str>) -> CoreResult<serde_json::Value> {
    let text = content.trim();
    let text = text
        .strip_prefix("```json")
        .or_else(|| text.strip_prefix("```"))
        .and_then(|t| t.strip_suffix("```"))
        .map(str::trim)
        .unwrap_or(text);
    let hint = if finish_reason == Some("length") { "（finish_reason=length，输出可能被 max_tokens 截断）" } else { "" };
    if text.is_empty() {
        return Err(CoreError::InvalidParameter(format!("LLM 返回内容为空，无法解析为 JSON{hint}")));
    }
    serde_json::from_str(text).map_err(|e| {
        let preview: String = text.chars().take(200).collect();
        CoreError::InvalidParameter(format!("LLM 返回内容不是合法 JSON{hint}：{e}；内容开头：{preview}"))
    })
}

/// 大模型对话扩展点
#[async_trait]
pub trait LlmProvider: Send + Sync {
    fn provider_id(&self) -> &str;

    async fn chat(&self, req: ChatRequest) -> CoreResult<ChatResponse>;

    /// 以 JSON 模式对话并把结果解析为 JSON
    async fn chat_json(&self, req: ChatRequest) -> CoreResult<serde_json::Value> {
        self.chat(req.json()).await?.json()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resp(content: &str, finish: Option<&str>) -> ChatResponse {
        ChatResponse { content: content.into(), model: "m".into(), usage: TokenUsage::default(), finish_reason: finish.map(Into::into) }
    }

    #[test]
    fn parses_json_content() {
        assert_eq!(resp(r#" {"a":1} "#, None).json().unwrap()["a"], 1);
        assert_eq!(resp("```json\n{\"a\":2}\n```", None).json().unwrap()["a"], 2);
    }

    #[test]
    fn invalid_json_is_clear_error() {
        let err = resp("not json", Some("stop")).json().unwrap_err().to_string();
        assert!(err.contains("不是合法 JSON") && err.contains("not json"), "{err}");
        let err = resp("   ", Some("stop")).json().unwrap_err().to_string();
        assert!(err.contains("为空"), "{err}");
        let err = resp("{\"a\":", Some("length")).json().unwrap_err().to_string();
        assert!(err.contains("截断"), "{err}");
    }

    #[test]
    fn serde_shapes() {
        assert_eq!(serde_json::to_value(ChatRole::Assistant).unwrap(), "assistant");
        assert_eq!(serde_json::to_value(ResponseFormat::JsonObject).unwrap(), "json_object");
        let req = ChatRequest::new(vec![ChatMessage::user("hi")]).json();
        assert_eq!(req.response_format, ResponseFormat::JsonObject);
    }
}
