//! leo-plugin-llm：大模型对话**注册层**插件（“大箱子”）。
//!
//! - 子插件（deepseek、以后的 qwen / openai / 本地模型……）只依赖 `leo-core`，在自己的 `apply` 里
//!   `ctx.register::<dyn LlmProvider>("deepseek", provider)` 注册驱动，不需要知道本插件存在。
//! - 本插件按配置设置默认驱动，并对外提供 [`LlmHub`] 服务。
//! - 配置了默认驱动但它没注册（插件被停用、拼写错误）时直接报错，不会悄悄换成别的驱动。
//!
//! - 对外暴露每个驱动的能力（[`LlmHub::capabilities`]：支持哪些接口风格 Chat Completions / Responses、
//!   各自是否支持 JsonObject / JsonSchema），请求可以用 `ChatRequest::api` 指定本次走哪种接口。
//! - 请求要 `JsonSchema`，但所选驱动 / 接口不支持时，按 `schema_fallback` 处理：
//!   `json_object`（默认）改用 JsonObject + 本地 Schema 校验，并打 warn 日志；`error` 直接报错。不会静默降级。
//!
//! 配置（catalog 的 `leo-plugins.json` 里 `"llm"` 一节）：
//! ```json
//! { "default": "deepseek", "schema_fallback": "json_object" }
//! ```

use std::sync::Arc;

use async_trait::async_trait;
use leo_core::context::Context;
use leo_core::error::{CoreError, CoreResult};
use leo_core::plugin::{Plugin, PluginCategory, PluginManifest};
use leo_core::registry::{DriverInfo, Registry};
use leo_core::spi::{
    ChatMessage, ChatRequest, ChatResponse, ChatRole, LlmApi, LlmCapabilities, LlmProvider, ResponseFormat, check_json_schema,
    from_json_value,
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

pub const PLUGIN_ID: &str = "leo-plugin-llm";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LlmRegistryConfig {
    /// 默认驱动 id；不填时用最早注册的驱动，填了但没注册时调用报错
    pub default: Option<String>,
    /// 请求 JsonSchema 但驱动 / 接口不支持时怎么办（默认 json_object）
    pub schema_fallback: SchemaFallback,
}

/// JsonSchema 不被支持时的处理方式
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SchemaFallback {
    /// 改用 JsonObject，把 Schema 写进系统提示词，返回后在本地按 Schema 校验（打 warn 日志）
    #[default]
    JsonObject,
    /// 直接报错
    Error,
}

/// 某个驱动的描述与能力
#[derive(Debug, Clone)]
pub struct LlmDriverInfo {
    pub driver: DriverInfo,
    pub capabilities: LlmCapabilities,
}

pub struct LlmRegistryPlugin {
    config: LlmRegistryConfig,
}

impl LlmRegistryPlugin {
    pub fn new(config: LlmRegistryConfig) -> Self {
        Self { config }
    }
}

impl Default for LlmRegistryPlugin {
    fn default() -> Self {
        Self::new(LlmRegistryConfig::default())
    }
}

#[async_trait]
impl Plugin for LlmRegistryPlugin {
    fn manifest(&self) -> PluginManifest {
        PluginManifest::new(PLUGIN_ID, "大模型对话注册层", env!("CARGO_PKG_VERSION"), PluginCategory::Registry)
            .with_description("管理全部 LLM 驱动子插件，按配置挑选默认驱动，对外提供统一的对话服务。")
            .with_capabilities(["registry:llm"])
    }

    async fn apply(&self, ctx: &Context) -> CoreResult<()> {
        let registry = ctx.registry::<dyn LlmProvider>();
        registry.set_default(self.config.default.as_deref());
        let r = registry.clone();
        ctx.on_dispose(move || r.set_default(None));
        ctx.provide(LlmHub { registry, schema_fallback: self.config.schema_fallback }).await;
        Ok(())
    }
}

/// LLM 统一服务
#[derive(Clone)]
pub struct LlmHub {
    registry: Registry<dyn LlmProvider>,
    schema_fallback: SchemaFallback,
}

/// 准备好的请求：可能已从 JsonSchema 降级为 JsonObject，降级时记下需要本地校验的 Schema
struct Prepared {
    req: ChatRequest,
    /// (schema 名, schema)：降级后需要本地校验
    local_schema: Option<(String, serde_json::Value)>,
}

impl LlmHub {
    /// 取本插件提供的服务；本插件没装配时直接用注册表（只是没有配置的默认驱动，schema_fallback 用默认 json_object）
    pub async fn of(ctx: &Context) -> Self {
        match ctx.inject::<LlmHub>().await {
            Ok(hub) => (*hub).clone(),
            Err(_) => LlmHub { registry: ctx.registry::<dyn LlmProvider>(), schema_fallback: SchemaFallback::default() },
        }
    }

    pub fn drivers(&self) -> Vec<DriverInfo> {
        self.registry.describe()
    }

    /// 全部驱动及其能力
    pub fn drivers_with_capabilities(&self) -> Vec<LlmDriverInfo> {
        self.registry
            .describe()
            .into_iter()
            .filter_map(|d| {
                let capabilities = self.registry.get(&d.id)?.capabilities();
                Some(LlmDriverInfo { driver: d, capabilities })
            })
            .collect()
    }

    /// 驱动能力；`None` 取默认驱动
    pub fn capabilities(&self, driver: Option<&str>) -> CoreResult<LlmCapabilities> {
        Ok(self.provider(driver)?.1.capabilities())
    }

    /// 当前 JsonSchema 降级策略
    pub fn schema_fallback(&self) -> SchemaFallback {
        self.schema_fallback
    }

    /// 取驱动；`None` 取默认驱动。配置的默认驱动没注册时报错
    pub fn provider(&self, id: Option<&str>) -> CoreResult<(String, Arc<dyn LlmProvider>)> {
        if id.is_none()
            && let Some(default) = self.registry.configured_default()
        {
            return self.registry.get(&default).map(|p| (default.clone(), p)).ok_or_else(|| {
                let registered = self.registry.ids();
                CoreError::ServiceNotFound(format!(
                    "默认 LLM 驱动 \"{default}\" 没有注册（已注册：{}）。请检查对应插件是否被停用，或修改 leo-plugins.json 的 llm.default",
                    if registered.is_empty() { "无".to_string() } else { registered.join(", ") }
                ))
            });
        }
        self.registry.resolve(id)
    }

    /// 按驱动能力检查接口与输出格式；JsonSchema 不被支持时按 `schema_fallback` 降级或报错
    fn prepare(&self, driver: &str, provider: &dyn LlmProvider, mut req: ChatRequest) -> CoreResult<Prepared> {
        let caps = provider.capabilities();
        let api = caps.effective_api(req.api);
        if !caps.supports_api(api) {
            let supported: Vec<&str> = caps.apis.iter().map(|a| a.api.as_str()).collect();
            return Err(CoreError::InvalidParameter(format!(
                "LLM 驱动 \"{driver}\" 不支持接口 {api}（支持：{}）",
                supported.join(", ")
            )));
        }
        if caps.supports_format(api, &req.response_format) {
            return Ok(Prepared { req, local_schema: None });
        }
        match std::mem::take(&mut req.response_format) {
            ResponseFormat::JsonSchema { name, schema, .. } => match self.schema_fallback {
                SchemaFallback::Error => Err(CoreError::InvalidParameter(format!(
                    "LLM 驱动 \"{driver}\" 的 {api} 接口不支持 json_schema 结构化输出（schema_fallback=error）。\
                     可改用支持的接口（如 api=responses），或把 llm.schema_fallback 设为 json_object"
                ))),
                SchemaFallback::JsonObject => {
                    if !caps.supports_format(api, &ResponseFormat::JsonObject) {
                        return Err(CoreError::InvalidParameter(format!(
                            "LLM 驱动 \"{driver}\" 的 {api} 接口既不支持 json_schema 也不支持 json_object，无法降级"
                        )));
                    }
                    tracing::warn!(
                        "LLM 驱动 \"{driver}\" 的 {api} 接口不支持 json_schema，按 schema_fallback=json_object 降级：\
                         改用 json_object，Schema \"{name}\" 写进系统提示词，返回后本地校验"
                    );
                    let hint = format!(
                        "请只输出一个 JSON 对象（json），必须符合下面这个名为 \"{name}\" 的 JSON Schema：\n{}",
                        serde_json::to_string(&schema).unwrap_or_default()
                    );
                    req.messages.insert(0, ChatMessage::new(ChatRole::System, hint));
                    req.response_format = ResponseFormat::JsonObject;
                    Ok(Prepared { req, local_schema: Some((name, schema)) })
                }
            },
            other => Err(CoreError::InvalidParameter(format!(
                "LLM 驱动 \"{driver}\" 的 {api} 接口不支持输出格式 {}",
                serde_json::to_string(&other).unwrap_or_default()
            ))),
        }
    }

    async fn run_chat(&self, driver: Option<&str>, req: ChatRequest) -> CoreResult<ChatResponse> {
        let (id, provider) = self.provider(driver)?;
        let prepared = self.prepare(&id, provider.as_ref(), req)?;
        let resp = provider.chat(prepared.req).await?;
        if let Some((name, schema)) = prepared.local_schema {
            check_json_schema(&resp.json()?, &schema, &name)?;
        }
        Ok(resp)
    }

    async fn run_chat_json(&self, driver: Option<&str>, req: ChatRequest) -> CoreResult<serde_json::Value> {
        let (id, provider) = self.provider(driver)?;
        let prepared = self.prepare(&id, provider.as_ref(), req.json())?;
        let value = provider.chat_json(prepared.req).await?;
        if let Some((name, schema)) = prepared.local_schema {
            check_json_schema(&value, &schema, &name)?;
        }
        Ok(value)
    }

    /// 用默认驱动对话（JsonSchema 降级时也会做本地校验）
    pub async fn chat(&self, req: ChatRequest) -> CoreResult<ChatResponse> {
        self.run_chat(None, req).await
    }

    /// 用默认驱动以 JSON 模式对话并解析结果（JsonSchema 时校验 Schema）
    pub async fn chat_json(&self, req: ChatRequest) -> CoreResult<serde_json::Value> {
        self.run_chat_json(None, req).await
    }

    /// 用默认驱动以 JSON 模式对话并反序列化为 `T`
    pub async fn chat_typed<T: DeserializeOwned>(&self, req: ChatRequest) -> CoreResult<T> {
        from_json_value(self.run_chat_json(None, req).await?)
    }

    /// 用指定驱动对话
    pub async fn chat_with(&self, driver: &str, req: ChatRequest) -> CoreResult<ChatResponse> {
        self.run_chat(Some(driver), req).await
    }

    /// 用指定驱动以 JSON 模式对话
    pub async fn chat_json_with(&self, driver: &str, req: ChatRequest) -> CoreResult<serde_json::Value> {
        self.run_chat_json(Some(driver), req).await
    }

    /// 用指定驱动以 JSON 模式对话并反序列化为 `T`
    pub async fn chat_typed_with<T: DeserializeOwned>(&self, driver: &str, req: ChatRequest) -> CoreResult<T> {
        from_json_value(self.run_chat_json(Some(driver), req).await?)
    }

    /// 本次请求实际会走的接口（驱动不存在时报错）
    pub fn effective_api(&self, driver: Option<&str>, req: &ChatRequest) -> CoreResult<LlmApi> {
        Ok(self.capabilities(driver)?.effective_api(req.api))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use leo_core::spi::{LlmApiCapabilities, TokenUsage};
    use std::sync::Mutex;

    struct Fake(&'static str);
    #[async_trait]
    impl LlmProvider for Fake {
        fn provider_id(&self) -> &str {
            self.0
        }
        async fn chat(&self, req: ChatRequest) -> CoreResult<ChatResponse> {
            let content = match req.response_format {
                ResponseFormat::Text => format!("{}|{}", self.0, req.messages[0].content),
                _ => format!(r#"{{"by":"{}"}}"#, self.0),
            };
            Ok(ChatResponse { content, model: "fake".into(), usage: TokenUsage::default(), finish_reason: Some("stop".into()) })
        }
    }

    /// 两种接口都有、只有 Responses 原生支持 JsonSchema 的驱动；记录收到的请求，按 `reply` 返回内容
    struct TwoApis {
        reply: &'static str,
        seen: Mutex<Vec<ChatRequest>>,
    }
    #[async_trait]
    impl LlmProvider for TwoApis {
        fn provider_id(&self) -> &str {
            "two"
        }
        fn capabilities(&self) -> LlmCapabilities {
            LlmCapabilities {
                default_api: LlmApi::ChatCompletions,
                apis: vec![
                    LlmApiCapabilities { api: LlmApi::ChatCompletions, json_object: true, json_schema: false },
                    LlmApiCapabilities { api: LlmApi::Responses, json_object: true, json_schema: true },
                ],
            }
        }
        async fn chat(&self, req: ChatRequest) -> CoreResult<ChatResponse> {
            let api = req.api.unwrap_or(LlmApi::ChatCompletions);
            if api == LlmApi::ChatCompletions && matches!(req.response_format, ResponseFormat::JsonSchema { .. }) {
                panic!("注册层不应该把 JsonSchema 发给不支持的接口");
            }
            self.seen.lock().unwrap().push(req);
            Ok(ChatResponse {
                content: self.reply.into(),
                model: "two".into(),
                usage: TokenUsage::default(),
                finish_reason: Some("stop".into()),
            })
        }
    }

    fn schema() -> serde_json::Value {
        serde_json::json!({"type": "object", "properties": {"speaker": {"type": "string"}}, "required": ["speaker"]})
    }

    async fn hub_with(reply: &'static str, fallback: SchemaFallback) -> (LlmHub, Arc<TwoApis>) {
        let root = Context::new("t");
        let two = Arc::new(TwoApis { reply, seen: Mutex::new(vec![]) });
        root.for_plugin("p").register::<dyn LlmProvider>("two", two.clone()).unwrap();
        LlmRegistryPlugin::new(LlmRegistryConfig { default: Some("two".into()), schema_fallback: fallback })
            .apply(&root.for_plugin(PLUGIN_ID))
            .await
            .unwrap();
        (LlmHub::of(&root).await, two)
    }

    #[tokio::test]
    async fn json_schema_native_on_responses_is_passed_through() {
        let (hub, two) = hub_with(r#"{"speaker":"师姐"}"#, SchemaFallback::Error).await;
        let req = ChatRequest::new(vec![ChatMessage::user("hi")]).json_schema("line", schema(), true).with_api(LlmApi::Responses);
        assert_eq!(hub.chat_json(req).await.unwrap()["speaker"], "师姐");
        let seen = two.seen.lock().unwrap();
        assert!(matches!(seen[0].response_format, ResponseFormat::JsonSchema { .. }));
        assert_eq!(seen[0].messages.len(), 1, "原生支持时不改提示词");
    }

    #[tokio::test]
    async fn json_schema_fallback_to_json_object_with_local_validation() {
        let (hub, two) = hub_with(r#"{"speaker":"师姐"}"#, SchemaFallback::JsonObject).await;
        let req = ChatRequest::new(vec![ChatMessage::user("hi")]).json_schema("line", schema(), true);
        assert_eq!(hub.chat_json(req.clone()).await.unwrap()["speaker"], "师姐");
        {
            let seen = two.seen.lock().unwrap();
            assert_eq!(seen[0].response_format, ResponseFormat::JsonObject);
            assert_eq!(seen[0].messages[0].role, ChatRole::System);
            assert!(seen[0].messages[0].content.contains("\"line\"") && seen[0].messages[0].content.contains("speaker"));
        }
        // chat()（非 JSON 接口）降级时同样本地校验
        assert_eq!(hub.chat(req).await.unwrap().content, r#"{"speaker":"师姐"}"#);

        #[derive(Deserialize)]
        struct Line {
            speaker: String,
        }
        let line: Line = hub
            .chat_typed(ChatRequest::new(vec![ChatMessage::user("hi")]).json_schema("line", schema(), false))
            .await
            .unwrap();
        assert_eq!(line.speaker, "师姐");
    }

    #[tokio::test]
    async fn json_schema_fallback_still_fails_on_invalid_output() {
        let (hub, _) = hub_with(r#"{"name":"师姐"}"#, SchemaFallback::JsonObject).await;
        let req = ChatRequest::new(vec![ChatMessage::user("hi")]).json_schema("line", schema(), true);
        let err = hub.chat_json(req.clone()).await.unwrap_err().to_string();
        assert!(err.contains("不符合 JSON Schema") && err.contains("speaker"), "{err}");
        let err = hub.chat(req).await.unwrap_err().to_string();
        assert!(err.contains("不符合 JSON Schema"), "{err}");
    }

    #[tokio::test]
    async fn json_schema_unsupported_with_error_policy_is_an_error() {
        let (hub, two) = hub_with(r#"{"speaker":"师姐"}"#, SchemaFallback::Error).await;
        let req = ChatRequest::new(vec![ChatMessage::user("hi")]).json_schema("line", schema(), true);
        let err = hub.chat_json(req).await.unwrap_err().to_string();
        assert!(err.contains("chat_completions") && err.contains("schema_fallback=error"), "{err}");
        assert!(two.seen.lock().unwrap().is_empty(), "报错时不应发出请求");
    }

    #[tokio::test]
    async fn unsupported_api_and_capabilities_are_exposed() {
        let root = Context::new("t");
        root.for_plugin("a").register::<dyn LlmProvider>("alpha", Arc::new(Fake("alpha"))).unwrap();
        LlmRegistryPlugin::default().apply(&root.for_plugin(PLUGIN_ID)).await.unwrap();
        let hub = LlmHub::of(&root).await;
        assert_eq!(hub.schema_fallback(), SchemaFallback::JsonObject);
        let caps = hub.capabilities(None).unwrap();
        assert_eq!(caps, LlmCapabilities::default());
        let err = hub
            .chat(ChatRequest::new(vec![ChatMessage::user("hi")]).with_api(LlmApi::Responses))
            .await
            .unwrap_err()
            .to_string();
        assert!(err.contains("不支持接口 responses") && err.contains("chat_completions"), "{err}");
        let infos = hub.drivers_with_capabilities();
        assert_eq!(infos.len(), 1);
        assert!(infos[0].driver.is_default && infos[0].driver.id == "alpha");
        assert_eq!(infos[0].capabilities, LlmCapabilities::default());
        assert_eq!(hub.effective_api(None, &ChatRequest::default()).unwrap(), LlmApi::ChatCompletions);
    }

    fn req() -> ChatRequest {
        ChatRequest::new(vec![ChatMessage::user("hi")])
    }

    #[tokio::test]
    async fn uses_configured_default_and_explicit_driver() {
        let root = Context::new("t");
        root.for_plugin("a").register::<dyn LlmProvider>("alpha", Arc::new(Fake("alpha"))).unwrap();
        root.for_plugin("b").register::<dyn LlmProvider>("beta", Arc::new(Fake("beta"))).unwrap();
        let hub_ctx = root.for_plugin(PLUGIN_ID);
        LlmRegistryPlugin::new(LlmRegistryConfig { default: Some("beta".into()), ..Default::default() }).apply(&hub_ctx).await.unwrap();
        let hub = LlmHub::of(&root).await;

        assert_eq!(hub.chat(req()).await.unwrap().content, "beta|hi");
        assert_eq!(hub.chat_with("alpha", req()).await.unwrap().content, "alpha|hi");
        assert_eq!(hub.chat_json(req()).await.unwrap()["by"], "beta");
        assert!(hub.drivers().iter().any(|d| d.id == "beta" && d.is_default));
    }

    #[tokio::test]
    async fn missing_configured_default_is_an_error() {
        let root = Context::new("t");
        root.for_plugin("a").register::<dyn LlmProvider>("alpha", Arc::new(Fake("alpha"))).unwrap();
        LlmRegistryPlugin::new(LlmRegistryConfig { default: Some("deepseek".into()), ..Default::default() })
            .apply(&root.for_plugin(PLUGIN_ID))
            .await
            .unwrap();
        let hub = LlmHub::of(&root).await;
        let err = hub.chat(req()).await.unwrap_err().to_string();
        assert!(err.contains("deepseek") && err.contains("alpha"), "{err}");
        let err = hub.chat_with("nope", req()).await.unwrap_err().to_string();
        assert!(err.contains("nope"), "{err}");
    }

    #[tokio::test]
    async fn no_drivers_is_an_error_and_no_default_uses_first() {
        let root = Context::new("t");
        LlmRegistryPlugin::default().apply(&root.for_plugin(PLUGIN_ID)).await.unwrap();
        let hub = LlmHub::of(&root).await;
        assert!(hub.chat(req()).await.is_err());
        root.for_plugin("a").register::<dyn LlmProvider>("alpha", Arc::new(Fake("alpha"))).unwrap();
        assert_eq!(hub.chat(req()).await.unwrap().content, "alpha|hi");
    }

    #[test]
    fn config_rejects_unknown_fields() {
        assert!(serde_json::from_str::<LlmRegistryConfig>(r#"{"defualt":"deepseek"}"#).is_err());
        let c: LlmRegistryConfig = serde_json::from_str(r#"{"default":"deepseek"}"#).unwrap();
        assert_eq!(c.default.as_deref(), Some("deepseek"));
        assert_eq!(c.schema_fallback, SchemaFallback::JsonObject);
        let c: LlmRegistryConfig = serde_json::from_str(r#"{"schema_fallback":"error"}"#).unwrap();
        assert_eq!(c.schema_fallback, SchemaFallback::Error);
        assert!(serde_json::from_str::<LlmRegistryConfig>(r#"{"schema_fallback":"ignore"}"#).is_err());
    }
}
