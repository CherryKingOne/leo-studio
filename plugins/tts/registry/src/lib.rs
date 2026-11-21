//! leo-plugin-tts：语音合成**注册层**插件（“大箱子”）。
//!
//! - 子插件（edge、以后的 cosyvoice / azure / 本地模型……）只依赖 `leo-core`，在自己的 `apply` 里
//!   `ctx.register::<dyn TTSProvider>("edge", provider)` 注册驱动，不需要知道本插件存在。
//! - 本插件按配置设置默认驱动，并对外提供 [`TtsHub`] 服务：按 `驱动id:声线` 前缀把合成请求路由到对应驱动。
//! - 注册顺序任意：子插件先装、后装、卸载，`TtsHub` 每次调用都读注册表的最新状态。
//!
//! 配置（catalog 的 `leo-plugins.json` 里 `"tts"` 一节）：
//! ```json
//! { "default": "edge" }
//! ```

use std::sync::Arc;

use async_trait::async_trait;
use leo_core::context::Context;
use leo_core::error::CoreResult;
use leo_core::plugin::{Plugin, PluginCategory, PluginManifest};
use leo_core::registry::{DriverInfo, Registry};
use leo_core::spi::{SynthesisOutput, SynthesisPayload, TTSProvider, VoiceInfo};
use serde::{Deserialize, Serialize};

pub const PLUGIN_ID: &str = "leo-plugin-tts";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TtsRegistryConfig {
    /// 默认驱动 id；不填或该驱动没注册时用最早注册的驱动
    pub default: Option<String>,
}

pub struct TtsRegistryPlugin {
    config: TtsRegistryConfig,
}

impl TtsRegistryPlugin {
    pub fn new(config: TtsRegistryConfig) -> Self {
        Self { config }
    }
}

impl Default for TtsRegistryPlugin {
    fn default() -> Self {
        Self::new(TtsRegistryConfig::default())
    }
}

#[async_trait]
impl Plugin for TtsRegistryPlugin {
    fn manifest(&self) -> PluginManifest {
        PluginManifest::new(PLUGIN_ID, "语音合成注册层", env!("CARGO_PKG_VERSION"), PluginCategory::Registry)
            .with_description("管理全部 TTS 驱动子插件，按配置挑选默认驱动，按“驱动id:声线”路由合成请求。")
            .with_capabilities(["registry:tts"])
    }

    async fn apply(&self, ctx: &Context) -> CoreResult<()> {
        let registry = ctx.registry::<dyn TTSProvider>();
        registry.set_default(self.config.default.as_deref());
        let r = registry.clone();
        ctx.on_dispose(move || r.set_default(None));
        ctx.provide(TtsHub { registry }).await;
        Ok(())
    }
}

/// 声线的路由结果
pub struct VoiceRoute {
    pub provider_id: String,
    pub provider: Arc<dyn TTSProvider>,
    /// 去掉驱动前缀后的声线 id
    pub voice: String,
}

/// 带驱动来源的声线
#[derive(Debug, Clone, Serialize)]
pub struct ProviderVoice {
    pub provider: String,
    /// 对外使用的声线 id：默认驱动的声线不带前缀，其他驱动的声线是 `驱动id:声线`
    pub id: String,
    #[serde(flatten)]
    pub info: VoiceInfo,
}

/// TTS 统一服务
#[derive(Clone)]
pub struct TtsHub {
    registry: Registry<dyn TTSProvider>,
}

impl TtsHub {
    /// 取本插件提供的服务；本插件没装配时直接用注册表（只是没有配置的默认驱动）
    pub async fn of(ctx: &Context) -> Self {
        match ctx.inject::<TtsHub>().await {
            Ok(hub) => (*hub).clone(),
            Err(_) => TtsHub { registry: ctx.registry::<dyn TTSProvider>() },
        }
    }

    pub fn drivers(&self) -> Vec<DriverInfo> {
        self.registry.describe()
    }

    /// 取驱动；`None` 取默认驱动
    pub fn provider(&self, id: Option<&str>) -> CoreResult<(String, Arc<dyn TTSProvider>)> {
        self.registry.resolve(id)
    }

    /// `edge:zh-CN-XiaoxiaoNeural` → edge 驱动 + `zh-CN-XiaoxiaoNeural`；没有已注册的前缀时交给默认驱动
    pub fn route(&self, voice: &str) -> CoreResult<VoiceRoute> {
        if let Some((prefix, rest)) = voice.split_once(':') {
            if let Some(provider) = self.registry.get(prefix) {
                return Ok(VoiceRoute { provider_id: prefix.to_string(), provider, voice: rest.to_string() });
            }
        }
        let (provider_id, provider) = self.registry.resolve(None)?;
        Ok(VoiceRoute { provider_id, provider, voice: voice.to_string() })
    }

    /// 合成：按声线前缀路由到驱动
    pub async fn synthesize(&self, payload: &SynthesisPayload) -> CoreResult<SynthesisOutput> {
        let route = self.route(&payload.voice)?;
        let payload = SynthesisPayload { voice: route.voice, ..payload.clone() };
        route.provider.synthesize(&payload).await
    }

    /// 全部驱动的声线（某个驱动失败时跳过它并记录警告）
    pub async fn list_voices(&self) -> CoreResult<Vec<ProviderVoice>> {
        let default_id = self.registry.default_entry().map(|(id, _)| id);
        let mut out = Vec::new();
        for (id, provider) in self.registry.entries() {
            match provider.list_voices().await {
                Ok(voices) => out.extend(voices.into_iter().map(|info| ProviderVoice {
                    provider: id.clone(),
                    id: if default_id.as_deref() == Some(id.as_str()) { info.id.clone() } else { format!("{id}:{}", info.id) },
                    info,
                })),
                Err(err) => tracing::warn!("TTS 驱动 {id} 获取声线失败：{err}"),
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use leo_core::ast::AudioFormat;

    struct Fake(&'static str);
    #[async_trait]
    impl TTSProvider for Fake {
        fn provider_id(&self) -> &str {
            self.0
        }
        async fn list_voices(&self) -> CoreResult<Vec<VoiceInfo>> {
            Ok(vec![VoiceInfo { id: "v1".into(), name: "v1".into(), gender: "".into(), locale: "".into(), description: "".into() }])
        }
        async fn synthesize(&self, p: &SynthesisPayload) -> CoreResult<SynthesisOutput> {
            Ok(SynthesisOutput { audio_bytes: format!("{}|{}", self.0, p.voice).into_bytes(), format: AudioFormat::Mp3, sample_rate: 0, duration_ms: 0 })
        }
    }

    #[tokio::test]
    async fn routes_by_prefix_and_default() {
        let root = Context::new("t");
        // 子插件先注册、注册层后装配，结果一样
        root.for_plugin("a").register::<dyn TTSProvider>("alpha", Arc::new(Fake("alpha"))).unwrap();
        root.for_plugin("b").register::<dyn TTSProvider>("beta", Arc::new(Fake("beta"))).unwrap();
        let hub_ctx = root.for_plugin(PLUGIN_ID);
        TtsRegistryPlugin::new(TtsRegistryConfig { default: Some("beta".into()) }).apply(&hub_ctx).await.unwrap();
        let hub = TtsHub::of(&root).await;

        let p = |voice: &str| SynthesisPayload { text: "x".into(), voice: voice.into(), speed: 1.0, pitch: 0.0, emotion: None };
        assert_eq!(hub.synthesize(&p("alpha:v1")).await.unwrap().audio_bytes, b"alpha|v1");
        assert_eq!(hub.synthesize(&p("v1")).await.unwrap().audio_bytes, b"beta|v1");
        let ids: Vec<_> = hub.list_voices().await.unwrap().into_iter().map(|v| v.id).collect();
        assert_eq!(ids, vec!["alpha:v1", "v1"]);

        hub_ctx.dispose();
        assert_eq!(root.registry::<dyn TTSProvider>().default_entry().unwrap().0, "alpha");
    }
}
