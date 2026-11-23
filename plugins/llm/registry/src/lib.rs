//! leo-plugin-llm：大模型对话**注册层**插件（“大箱子”）。
//!
//! - 子插件（deepseek、以后的 qwen / openai / 本地模型……）只依赖 `leo-core`，在自己的 `apply` 里
//!   `ctx.register::<dyn LlmProvider>("deepseek", provider)` 注册驱动，不需要知道本插件存在。
//! - 本插件按配置设置默认驱动，并对外提供 [`LlmHub`] 服务。
//! - 配置了默认驱动但它没注册（插件被停用、拼写错误）时直接报错，不会悄悄换成别的驱动。
//!
//! 配置（catalog 的 `leo-plugins.json` 里 `"llm"` 一节）：
//! ```json
//! { "default": "deepseek" }
//! ```

use std::sync::Arc;

use async_trait::async_trait;
use leo_core::context::Context;
use leo_core::error::{CoreError, CoreResult};
use leo_core::plugin::{Plugin, PluginCategory, PluginManifest};
use leo_core::registry::{DriverInfo, Registry};
use leo_core::spi::{ChatRequest, ChatResponse, LlmProvider};
use serde::{Deserialize, Serialize};

pub const PLUGIN_ID: &str = "leo-plugin-llm";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LlmRegistryConfig {
    /// 默认驱动 id；不填时用最早注册的驱动，填了但没注册时调用报错
    pub default: Option<String>,
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
        ctx.provide(LlmHub { registry }).await;
        Ok(())
    }
}

/// LLM 统一服务
#[derive(Clone)]
pub struct LlmHub {
    registry: Registry<dyn LlmProvider>,
}

impl LlmHub {
    /// 取本插件提供的服务；本插件没装配时直接用注册表（只是没有配置的默认驱动）
    pub async fn of(ctx: &Context) -> Self {
        match ctx.inject::<LlmHub>().await {
            Ok(hub) => (*hub).clone(),
            Err(_) => LlmHub { registry: ctx.registry::<dyn LlmProvider>() },
        }
    }

    pub fn drivers(&self) -> Vec<DriverInfo> {
        self.registry.describe()
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

    /// 用默认驱动对话
    pub async fn chat(&self, req: ChatRequest) -> CoreResult<ChatResponse> {
        self.provider(None)?.1.chat(req).await
    }

    /// 用默认驱动以 JSON 模式对话并解析结果
    pub async fn chat_json(&self, req: ChatRequest) -> CoreResult<serde_json::Value> {
        self.provider(None)?.1.chat_json(req).await
    }

    /// 用指定驱动对话
    pub async fn chat_with(&self, driver: &str, req: ChatRequest) -> CoreResult<ChatResponse> {
        self.provider(Some(driver))?.1.chat(req).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use leo_core::spi::{ChatMessage, TokenUsage};

    struct Fake(&'static str);
    #[async_trait]
    impl LlmProvider for Fake {
        fn provider_id(&self) -> &str {
            self.0
        }
        async fn chat(&self, req: ChatRequest) -> CoreResult<ChatResponse> {
            let content = match req.response_format {
                leo_core::spi::ResponseFormat::JsonObject => format!(r#"{{"by":"{}"}}"#, self.0),
                leo_core::spi::ResponseFormat::Text => format!("{}|{}", self.0, req.messages[0].content),
            };
            Ok(ChatResponse { content, model: "fake".into(), usage: TokenUsage::default(), finish_reason: Some("stop".into()) })
        }
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
        LlmRegistryPlugin::new(LlmRegistryConfig { default: Some("beta".into()) }).apply(&hub_ctx).await.unwrap();
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
        LlmRegistryPlugin::new(LlmRegistryConfig { default: Some("deepseek".into()) })
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
    }
}
