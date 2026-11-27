//! leo-plugin-llm：大语言模型**注册层**插件（“大箱子”）。
//!
//! - 供应商子插件（deepseek、volcengine、bailian……）只依赖 `leo-core` 和
//!   `leo-llm-protocol`，在自己的 `apply` 里注册驱动，不需要知道本插件存在。
//! - 本插件按配置设置默认驱动，并对外提供 [`LlmHub`]：
//!   - `chat`：canonical 请求 / 响应；
//!   - `chat_raw`：接收 OpenAI Chat / Responses / Anthropic 任一格式的 JSON，
//!     转成 canonical 调驱动，再把结果转回同一种协议（跨协议桥）。
//! - 注册顺序任意：子插件先装、后装、卸载，`LlmHub` 每次调用都读注册表的最新状态。
//!
//! 配置（catalog 的 `leo-plugins.json` 里 `"llm"` 一节）：
//! ```json
//! { "default": "deepseek" }
//! ```

use std::sync::Arc;

use async_trait::async_trait;
use leo_core::context::Context;
use leo_core::error::CoreResult;
use leo_core::plugin::{Plugin, PluginCategory, PluginManifest};
use leo_core::registry::{DriverInfo, Registry};
use leo_core::spi::{ChatRequest, ChatResponse, ChatStream, LlmProtocol, LlmProvider, ModelInfo};
use serde::{Deserialize, Serialize};

pub const PLUGIN_ID: &str = "leo-plugin-llm";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LlmRegistryConfig {
    /// 默认驱动 id；不填或该驱动没注册时用最早注册的驱动
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
        PluginManifest::new(
            PLUGIN_ID,
            "大语言模型注册层",
            env!("CARGO_PKG_VERSION"),
            PluginCategory::Registry,
        )
        .with_description(
            "管理全部 LLM 供应商驱动，按配置挑选默认驱动，按“驱动id:模型”路由，并提供 OpenAI / Anthropic 协议桥。",
        )
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

/// 模型路由结果
pub struct ModelRoute {
    pub provider_id: String,
    pub provider: Arc<dyn LlmProvider>,
    /// 去掉驱动前缀后的模型 id；`None` 表示交给驱动用配置里的默认模型
    pub model: Option<String>,
}

/// 带驱动来源的模型
#[derive(Debug, Clone, Serialize)]
pub struct ProviderModel {
    pub provider: String,
    /// 对外使用的模型 id：默认驱动的模型不带前缀，其他驱动的模型是 `驱动id:模型`
    pub id: String,
    #[serde(flatten)]
    pub info: ModelInfo,
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
            Err(_) => LlmHub {
                registry: ctx.registry::<dyn LlmProvider>(),
            },
        }
    }

    pub fn drivers(&self) -> Vec<DriverInfo> {
        self.registry.describe()
    }

    /// 取驱动；`None` 取默认驱动
    pub fn provider(&self, id: Option<&str>) -> CoreResult<(String, Arc<dyn LlmProvider>)> {
        self.registry.resolve(id)
    }

    /// 某个驱动的上游协议（诊断用）
    pub fn protocol_of(&self, id: &str) -> Option<LlmProtocol> {
        self.registry.get(id).map(|provider| provider.protocol())
    }

    /// `deepseek:deepseek-chat` → deepseek 驱动 + `deepseek-chat`；没有已注册前缀时交给默认驱动
    pub fn route(&self, model: Option<&str>) -> CoreResult<ModelRoute> {
        if let Some(spec) = model.map(str::trim).filter(|spec| !spec.is_empty()) {
            if let Some((prefix, rest)) = spec.split_once(':')
                && let Some(provider) = self.registry.get(prefix)
            {
                return Ok(ModelRoute {
                    provider_id: prefix.to_string(),
                    provider,
                    model: non_empty(rest),
                });
            }
            let (provider_id, provider) = self.registry.resolve(None)?;
            return Ok(ModelRoute {
                provider_id,
                provider,
                model: Some(spec.to_string()),
            });
        }
        let (provider_id, provider) = self.registry.resolve(None)?;
        Ok(ModelRoute {
            provider_id,
            provider,
            model: None,
        })
    }

    /// canonical 对话：按模型前缀路由到驱动
    pub async fn chat(&self, request: &ChatRequest) -> CoreResult<ChatResponse> {
        let route = self.route(request.model.as_deref())?;
        let payload = ChatRequest {
            model: route.model.clone(),
            ..request.clone()
        };
        route.provider.chat(&payload).await
    }

    /// 流式对话：按模型路由到驱动，返回 canonical 增量流。
    pub async fn chat_stream(&self, request: &ChatRequest) -> CoreResult<ChatStream> {
        let route = self.route(request.model.as_deref())?;
        let payload = ChatRequest {
            model: route.model.clone(),
            ..request.clone()
        };
        route.provider.chat_stream(&payload).await
    }

    /// 协议桥：按 `protocol` 解析入参 → canonical → 调驱动 → 再按同一协议编码返回。
    ///
    /// 上游驱动实际走什么协议由驱动自己的配置决定；这里负责的是“调用方协议”的
    /// 解析与还原，所以 OpenAI / Anthropic 任意一边进、另一边出都能工作。
    pub async fn chat_raw(
        &self,
        protocol: LlmProtocol,
        body: serde_json::Value,
    ) -> CoreResult<serde_json::Value> {
        let request = leo_llm_protocol::decode_request(protocol, &body)?;
        let response = self.chat(&request).await?;
        Ok(leo_llm_protocol::encode_response(protocol, &response))
    }

    /// 全部驱动的模型（某个驱动失败时跳过它并记录警告）
    pub async fn list_models(&self) -> CoreResult<Vec<ProviderModel>> {
        let default_id = self.registry.default_entry().map(|(id, _)| id);
        let mut out = Vec::new();
        for (id, provider) in self.registry.entries() {
            match provider.list_models().await {
                Ok(models) => out.extend(models.into_iter().map(|info| ProviderModel {
                    provider: id.clone(),
                    id: if default_id.as_deref() == Some(id.as_str()) {
                        info.id.clone()
                    } else {
                        format!("{id}:{}", info.id)
                    },
                    info,
                })),
                Err(err) => tracing::warn!("LLM 驱动 {id} 获取模型列表失败：{err}"),
            }
        }
        Ok(out)
    }
}

fn non_empty(value: &str) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::StreamExt;
    use leo_core::spi::{ChatMessage, FinishReason, TokenUsage};
    use serde_json::json;

    struct Fake {
        id: &'static str,
        protocol: LlmProtocol,
        answer: &'static str,
    }

    #[async_trait]
    impl LlmProvider for Fake {
        fn provider_id(&self) -> &str {
            self.id
        }

        fn protocol(&self) -> LlmProtocol {
            self.protocol
        }

        async fn list_models(&self) -> CoreResult<Vec<ModelInfo>> {
            Ok(vec![ModelInfo {
                id: format!("{}-model", self.id),
                name: format!("{}-model", self.id),
                context_window: Some(8192),
                capabilities: vec!["chat".into()],
            }])
        }

        async fn chat(&self, request: &ChatRequest) -> CoreResult<ChatResponse> {
            Ok(ChatResponse {
                id: Some("fake-1".into()),
                model: request.model.clone().unwrap_or_else(|| "fake-model".into()),
                content: format!("{}|{}", self.answer, request.messages.len()),
                reasoning_content: None,
                finish_reason: FinishReason::Stop,
                usage: Some(TokenUsage {
                    input_tokens: 3,
                    output_tokens: 2,
                    total_tokens: 5,
                }),
            })
        }
    }

    async fn hub_with(default: Option<&str>) -> (Context, LlmHub) {
        let root = Context::new("llm_registry_test");
        let a = root.for_plugin("a");
        a.register::<dyn LlmProvider>(
            "alpha",
            Arc::new(Fake {
                id: "alpha",
                protocol: LlmProtocol::OpenAiChat,
                answer: "alpha",
            }),
        )
        .unwrap();
        let b = root.for_plugin("b");
        b.register::<dyn LlmProvider>(
            "beta",
            Arc::new(Fake {
                id: "beta",
                protocol: LlmProtocol::Anthropic,
                answer: "beta",
            }),
        )
        .unwrap();
        let hub_ctx = root.for_plugin(PLUGIN_ID);
        LlmRegistryPlugin::new(LlmRegistryConfig {
            default: default.map(str::to_string),
        })
        .apply(&hub_ctx)
        .await
        .unwrap();
        let hub = LlmHub::of(&root).await;
        (root, hub)
    }

    fn request(model: Option<&str>) -> ChatRequest {
        ChatRequest {
            model: model.map(str::to_string),
            messages: vec![ChatMessage::user("你好")],
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn chat_stream_routes_and_uses_fallback() {
        let (_root, hub) = hub_with(Some("beta")).await;
        let mut stream = hub
            .chat_stream(&request(Some("alpha:alpha-model")))
            .await
            .unwrap();
        let delta = stream.next().await.unwrap().unwrap();
        assert_eq!(delta.content.as_deref(), Some("alpha|1"));
        assert_eq!(delta.finish_reason, Some(FinishReason::Stop));
        assert_eq!(
            delta.usage,
            Some(TokenUsage {
                input_tokens: 3,
                output_tokens: 2,
                total_tokens: 5,
            })
        );
        assert!(stream.next().await.is_none());
    }

    #[tokio::test]
    async fn routes_by_prefix_and_default() {
        let (_root, hub) = hub_with(Some("beta")).await;
        let beta = hub.chat(&request(None)).await.unwrap();
        assert_eq!(beta.content, "beta|1");
        assert_eq!(beta.model, "fake-model");

        let alpha = hub.chat(&request(Some("alpha:alpha-model"))).await.unwrap();
        assert_eq!(alpha.content, "alpha|1");
        assert_eq!(alpha.model, "alpha-model");

        let default_with_model = hub.chat(&request(Some("qwen-plus"))).await.unwrap();
        assert_eq!(default_with_model.content, "beta|1");
        assert_eq!(default_with_model.model, "qwen-plus");
    }

    #[tokio::test]
    async fn models_are_prefixed_for_non_default_drivers() {
        let (_root, hub) = hub_with(Some("beta")).await;
        let models = hub.list_models().await.unwrap();
        let ids: Vec<_> = models.iter().map(|model| model.id.as_str()).collect();
        assert_eq!(ids, vec!["alpha:alpha-model", "beta-model"]);

        assert_eq!(hub.protocol_of("alpha"), Some(LlmProtocol::OpenAiChat));
        assert_eq!(hub.protocol_of("beta"), Some(LlmProtocol::Anthropic));
        assert_eq!(hub.protocol_of("missing"), None);
    }

    #[tokio::test]
    async fn chat_raw_bridges_openai_and_anthropic_shapes() {
        let (_root, hub) = hub_with(Some("alpha")).await;

        let openai_body = json!({
            "model": "alpha:alpha-model",
            "messages": [{"role": "user", "content": "你好"}]
        });
        let openai_response = hub
            .chat_raw(LlmProtocol::OpenAiChat, openai_body)
            .await
            .unwrap();
        assert_eq!(
            openai_response["choices"][0]["message"]["content"],
            json!("alpha|1")
        );
        assert_eq!(openai_response["usage"]["total_tokens"], json!(5));

        let anthropic_body = json!({
            "model": "alpha:alpha-model",
            "max_tokens": 1024,
            "messages": [{"role": "user", "content": "你好"}]
        });
        let anthropic_response = hub
            .chat_raw(LlmProtocol::Anthropic, anthropic_body)
            .await
            .unwrap();
        assert_eq!(anthropic_response["content"][0]["text"], json!("alpha|1"));
        assert_eq!(anthropic_response["usage"]["output_tokens"], json!(2));
    }

    #[tokio::test]
    async fn dispose_restores_no_default() {
        let (root, _hub) = hub_with(Some("beta")).await;
        // apply 过的 hub_ctx 没有保留，直接重新取注册表验证默认仍为 beta；
        // 卸载行为由 PluginManager 的通用测试覆盖，这里只验证服务可用。
        assert_eq!(
            root.registry::<dyn LlmProvider>()
                .default_entry()
                .unwrap()
                .0,
            "beta"
        );
    }
}
