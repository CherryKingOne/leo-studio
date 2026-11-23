use async_trait::async_trait;
use serde::de::DeserializeOwned;
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
///
/// 序列化形状：`"text"` / `"json_object"` / `{"json_schema": {"name": .., "schema": {..}, "strict": true}}`
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResponseFormat {
    #[default]
    Text,
    /// 要求模型输出合法 JSON 对象（提示词里需要说明要 JSON）
    JsonObject,
    /// 结构化输出：要求模型输出符合给定 JSON Schema 的 JSON。
    /// 驱动 / 接口不支持时由注册层按 `schema_fallback` 降级为 JsonObject + 本地校验，或报错
    JsonSchema {
        /// Schema 名称（DeepSeek Responses 接口必填）
        name: String,
        /// JSON Schema（对象）
        schema: serde_json::Value,
        /// 是否要求服务端严格遵守 Schema
        #[serde(default)]
        strict: bool,
    },
}

impl ResponseFormat {
    /// 构造结构化输出格式
    pub fn json_schema(name: impl Into<String>, schema: serde_json::Value, strict: bool) -> Self {
        Self::JsonSchema { name: name.into(), schema, strict }
    }

    /// 是否要求 JSON 输出（JsonObject / JsonSchema）
    pub fn is_json(&self) -> bool {
        !matches!(self, Self::Text)
    }

    /// JsonSchema 时返回 Schema
    pub fn schema(&self) -> Option<&serde_json::Value> {
        match self {
            Self::JsonSchema { schema, .. } => Some(schema),
            _ => None,
        }
    }
}

/// 接口风格。同一家模型服务可能同时提供多种接口（如 DeepSeek 的 Chat Completions 与 Responses）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LlmApi {
    /// OpenAI 兼容 Chat Completions（`messages` / `response_format`）
    ChatCompletions,
    /// OpenAI 兼容 Responses（`input` / `text.format`）
    Responses,
}

impl LlmApi {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ChatCompletions => "chat_completions",
            Self::Responses => "responses",
        }
    }
}

impl std::fmt::Display for LlmApi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// 某个接口风格支持的输出格式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LlmApiCapabilities {
    pub api: LlmApi,
    /// 支持 `ResponseFormat::JsonObject`
    pub json_object: bool,
    /// 服务端原生支持 `ResponseFormat::JsonSchema`
    pub json_schema: bool,
}

/// 驱动能力：支持哪些接口风格、每种接口支持哪些输出格式
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LlmCapabilities {
    /// 请求没指定 `api` 时驱动使用的接口
    pub default_api: LlmApi,
    pub apis: Vec<LlmApiCapabilities>,
}

impl Default for LlmCapabilities {
    /// 老驱动的默认能力：只有 Chat Completions，支持 JsonObject，不支持 JsonSchema
    fn default() -> Self {
        Self {
            default_api: LlmApi::ChatCompletions,
            apis: vec![LlmApiCapabilities { api: LlmApi::ChatCompletions, json_object: true, json_schema: false }],
        }
    }
}

impl LlmCapabilities {
    /// 某个接口的能力；驱动不支持该接口时返回 None
    pub fn api(&self, api: LlmApi) -> Option<&LlmApiCapabilities> {
        self.apis.iter().find(|a| a.api == api)
    }

    pub fn supports_api(&self, api: LlmApi) -> bool {
        self.api(api).is_some()
    }

    /// 请求实际会走的接口（请求里的 `api` 优先，否则驱动默认）
    pub fn effective_api(&self, requested: Option<LlmApi>) -> LlmApi {
        requested.unwrap_or(self.default_api)
    }

    /// 该接口是否原生支持某种输出格式
    pub fn supports_format(&self, api: LlmApi, format: &ResponseFormat) -> bool {
        match (self.api(api), format) {
            (None, _) => false,
            (Some(_), ResponseFormat::Text) => true,
            (Some(a), ResponseFormat::JsonObject) => a.json_object,
            (Some(a), ResponseFormat::JsonSchema { .. }) => a.json_schema,
        }
    }

    /// 支持的接口列表
    pub fn api_list(&self) -> Vec<LlmApi> {
        self.apis.iter().map(|a| a.api).collect()
    }
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
    /// 覆盖驱动配置的接口风格；`None` 用驱动默认
    #[serde(default)]
    pub api: Option<LlmApi>,
}

impl ChatRequest {
    pub fn new(messages: Vec<ChatMessage>) -> Self {
        Self { messages, ..Default::default() }
    }

    /// 要求 JSON 输出；已经是 JsonSchema 时保持不变
    pub fn json(mut self) -> Self {
        if !self.response_format.is_json() {
            self.response_format = ResponseFormat::JsonObject;
        }
        self
    }

    /// 要求输出符合 JSON Schema 的 JSON
    pub fn json_schema(mut self, name: impl Into<String>, schema: serde_json::Value, strict: bool) -> Self {
        self.response_format = ResponseFormat::json_schema(name, schema, strict);
        self
    }

    /// 指定本次请求的接口风格
    pub fn with_api(mut self, api: LlmApi) -> Self {
        self.api = Some(api);
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

/// 按 JSON Schema 校验 JSON 值（轻量子集，见 [`validate_json_schema`]），失败时返回清楚的错误
pub fn check_json_schema(value: &serde_json::Value, schema: &serde_json::Value, name: &str) -> CoreResult<()> {
    validate_json_schema(value, schema).map_err(|errors| {
        CoreError::InvalidParameter(format!("LLM 输出不符合 JSON Schema \"{name}\"：{}", errors.join("；")))
    })
}

/// 轻量 JSON Schema 校验，支持：`type`（含类型数组、integer/number）、`required`、`properties`、
/// `additionalProperties: false`、`items`（单个 Schema）、`enum`、`const`。
/// 不认识的关键字忽略（不报错，也不校验）。返回全部不符合项（带 JSON 路径，如 `$.items[0].name`）
pub fn validate_json_schema(value: &serde_json::Value, schema: &serde_json::Value) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    validate_at(value, schema, "$", &mut errors);
    if errors.is_empty() { Ok(()) } else { Err(errors) }
}

fn type_name(v: &serde_json::Value) -> &'static str {
    use serde_json::Value;
    match v {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(n) if n.is_i64() || n.is_u64() => "integer",
        Value::Number(n) if n.as_f64().is_some_and(|f| f.fract() == 0.0) => "integer",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

fn type_matches(v: &serde_json::Value, ty: &str) -> bool {
    let actual = type_name(v);
    actual == ty || (ty == "number" && actual == "integer")
}

fn validate_at(value: &serde_json::Value, schema: &serde_json::Value, path: &str, errors: &mut Vec<String>) {
    use serde_json::Value;
    let Some(schema) = schema.as_object() else { return };

    if let Some(ty) = schema.get("type") {
        let allowed: Vec<&str> = match ty {
            Value::String(t) => vec![t.as_str()],
            Value::Array(ts) => ts.iter().filter_map(Value::as_str).collect(),
            _ => vec![],
        };
        if !allowed.is_empty() && !allowed.iter().any(|t| type_matches(value, t)) {
            errors.push(format!("{path} 类型应为 {}，实际为 {}", allowed.join(" | "), type_name(value)));
            return;
        }
    }
    if let Some(Value::Array(options)) = schema.get("enum")
        && !options.contains(value)
    {
        errors.push(format!("{path} 取值 {value} 不在 enum {} 中", Value::Array(options.clone())));
    }
    if let Some(expected) = schema.get("const")
        && expected != value
    {
        errors.push(format!("{path} 取值应为 {expected}，实际为 {value}"));
    }

    if let Value::Object(obj) = value {
        if let Some(Value::Array(required)) = schema.get("required") {
            for key in required.iter().filter_map(Value::as_str) {
                if !obj.contains_key(key) {
                    errors.push(format!("{path} 缺少必填字段 \"{key}\""));
                }
            }
        }
        let props = schema.get("properties").and_then(Value::as_object);
        if let Some(props) = props {
            for (key, sub) in props {
                if let Some(v) = obj.get(key) {
                    validate_at(v, sub, &format!("{path}.{key}"), errors);
                }
            }
        }
        if schema.get("additionalProperties") == Some(&Value::Bool(false)) {
            for key in obj.keys() {
                if !props.is_some_and(|p| p.contains_key(key)) {
                    errors.push(format!("{path} 不允许出现额外字段 \"{key}\""));
                }
            }
        }
    }
    if let (Value::Array(items), Some(item_schema @ Value::Object(_))) = (value, schema.get("items")) {
        for (i, item) in items.iter().enumerate() {
            validate_at(item, item_schema, &format!("{path}[{i}]"), errors);
        }
    }
}

/// 大模型对话扩展点
#[async_trait]
pub trait LlmProvider: Send + Sync {
    fn provider_id(&self) -> &str;

    /// 驱动能力（支持的接口风格与输出格式）。默认：只有 Chat Completions + JsonObject，老驱动不用改
    fn capabilities(&self) -> LlmCapabilities {
        LlmCapabilities::default()
    }

    async fn chat(&self, req: ChatRequest) -> CoreResult<ChatResponse>;

    /// 以 JSON 模式对话并把结果解析为 JSON。
    /// - `Text` 自动改为 `JsonObject`；`JsonObject` / `JsonSchema` 原样发送
    /// - `JsonSchema` 时再按 Schema 本地校验，不符合返回清楚的错误
    async fn chat_json(&self, req: ChatRequest) -> CoreResult<serde_json::Value> {
        let req = req.json();
        let schema = match &req.response_format {
            ResponseFormat::JsonSchema { name, schema, .. } => Some((name.clone(), schema.clone())),
            _ => None,
        };
        let value = self.chat(req).await?.json()?;
        if let Some((name, schema)) = schema {
            check_json_schema(&value, &schema, &name)?;
        }
        Ok(value)
    }
}

/// 以 JSON 模式对话并反序列化为 `T`（`JsonSchema` 时先做 Schema 校验）
pub async fn chat_typed<T: DeserializeOwned>(provider: &dyn LlmProvider, req: ChatRequest) -> CoreResult<T> {
    let value = provider.chat_json(req).await?;
    from_json_value(value)
}

/// 把 JSON 值反序列化为 `T`，失败时返回清楚的错误
pub fn from_json_value<T: DeserializeOwned>(value: serde_json::Value) -> CoreResult<T> {
    let preview: String = value.to_string().chars().take(200).collect();
    serde_json::from_value(value).map_err(|e| {
        CoreError::InvalidParameter(format!(
            "LLM 输出无法转换为 {}：{e}；内容开头：{preview}",
            std::any::type_name::<T>()
        ))
    })
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
        assert_eq!(serde_json::to_value(LlmApi::ChatCompletions).unwrap(), "chat_completions");
        assert_eq!(serde_json::from_value::<LlmApi>(serde_json::json!("responses")).unwrap(), LlmApi::Responses);
        let f = ResponseFormat::json_schema("s", serde_json::json!({"type": "object"}), true);
        let v = serde_json::to_value(&f).unwrap();
        assert_eq!(v, serde_json::json!({"json_schema": {"name": "s", "schema": {"type": "object"}, "strict": true}}));
        assert_eq!(serde_json::from_value::<ResponseFormat>(v).unwrap(), f);
        // 老格式的请求（没有 api 字段）照样能反序列化
        let old: ChatRequest = serde_json::from_str(r#"{"model":null,"messages":[],"temperature":null,"max_tokens":null}"#).unwrap();
        assert_eq!(old.api, None);
        assert_eq!(old.response_format, ResponseFormat::Text);
        // json() 不覆盖已有的 JsonSchema
        let req = ChatRequest::default().json_schema("s", serde_json::json!({}), false).json().with_api(LlmApi::Responses);
        assert!(matches!(req.response_format, ResponseFormat::JsonSchema { .. }));
        assert_eq!(req.api, Some(LlmApi::Responses));
    }

    fn schema() -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "speaker": {"type": "string"},
                "confidence": {"type": "number"},
                "tags": {"type": "array", "items": {"type": "string", "enum": ["a", "b"]}},
                "kind": {"const": "line"}
            },
            "required": ["speaker", "confidence"],
            "additionalProperties": false
        })
    }

    #[test]
    fn schema_validation_accepts_valid_and_reports_paths() {
        let ok = serde_json::json!({"speaker": "师姐", "confidence": 1, "tags": ["a"], "kind": "line"});
        assert!(validate_json_schema(&ok, &schema()).is_ok());

        let bad = serde_json::json!({"confidence": "high", "tags": ["a", "c"], "extra": 1, "kind": "x"});
        let errors = validate_json_schema(&bad, &schema()).unwrap_err();
        let all = errors.join("\n");
        assert!(all.contains("缺少必填字段 \"speaker\""), "{all}");
        assert!(all.contains("$.confidence 类型应为 number"), "{all}");
        assert!(all.contains("$.tags[1]"), "{all}");
        assert!(all.contains("额外字段 \"extra\""), "{all}");
        assert!(all.contains("$.kind"), "{all}");

        let err = check_json_schema(&serde_json::json!([1]), &schema(), "speaker").unwrap_err().to_string();
        assert!(err.contains("不符合 JSON Schema \"speaker\"") && err.contains("类型应为 object"), "{err}");
        // 类型数组与 integer ⊂ number
        let s = serde_json::json!({"type": ["integer", "null"]});
        assert!(validate_json_schema(&serde_json::json!(3), &s).is_ok());
        assert!(validate_json_schema(&serde_json::json!(null), &s).is_ok());
        assert!(validate_json_schema(&serde_json::json!(3.5), &s).is_err());
    }

    #[test]
    fn capabilities_defaults_and_queries() {
        let c = LlmCapabilities::default();
        assert_eq!(c.default_api, LlmApi::ChatCompletions);
        assert!(c.supports_format(LlmApi::ChatCompletions, &ResponseFormat::JsonObject));
        assert!(!c.supports_format(LlmApi::ChatCompletions, &ResponseFormat::json_schema("s", serde_json::json!({}), true)));
        assert!(!c.supports_api(LlmApi::Responses));
        assert_eq!(c.effective_api(None), LlmApi::ChatCompletions);
        assert_eq!(c.effective_api(Some(LlmApi::Responses)), LlmApi::Responses);
    }

    struct Fixed(&'static str);
    #[async_trait]
    impl LlmProvider for Fixed {
        fn provider_id(&self) -> &str {
            "fixed"
        }
        async fn chat(&self, req: ChatRequest) -> CoreResult<ChatResponse> {
            assert!(req.response_format.is_json());
            Ok(resp(self.0, Some("stop")))
        }
    }

    #[derive(Debug, serde::Deserialize)]
    struct Line {
        speaker: String,
        confidence: f64,
    }

    #[tokio::test]
    async fn chat_json_validates_schema_and_chat_typed_deserializes() {
        let good = Fixed(r#"{"speaker":"师姐","confidence":0.9}"#);
        let req = ChatRequest::new(vec![ChatMessage::user("hi")]).json_schema("line", schema(), true);
        let line: Line = chat_typed(&good, req.clone()).await.unwrap();
        assert_eq!((line.speaker.as_str(), line.confidence), ("师姐", 0.9));

        let bad = Fixed(r#"{"speaker":"师姐"}"#);
        let err = bad.chat_json(req).await.unwrap_err().to_string();
        assert!(err.contains("缺少必填字段 \"confidence\""), "{err}");
        // JsonObject 不做 Schema 校验，但 chat_typed 反序列化失败时报清楚的错误
        let err = chat_typed::<Line>(&bad, ChatRequest::new(vec![ChatMessage::user("hi")])).await.unwrap_err().to_string();
        assert!(err.contains("无法转换为") && err.contains("confidence"), "{err}");
    }
}
