use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use leo_core::ast::ScriptDocument;
use leo_core::context::Context;
use leo_core::error::{CoreError, CoreResult};
use leo_core::plugin::{Exporter, Plugin, PluginCategory, PluginManager, PluginManifest};

struct NamedExporter(&'static str);

#[async_trait]
impl Exporter for NamedExporter {
    fn format_name(&self) -> &str {
        self.0
    }

    fn mime_type(&self) -> &str {
        "text/plain"
    }

    async fn export(&self, _script: &ScriptDocument) -> CoreResult<Vec<u8>> {
        Ok(self.0.as_bytes().to_vec())
    }
}

struct ExporterPlugin {
    format: &'static str,
    applied: Arc<Mutex<Vec<String>>>,
}

#[async_trait]
impl Plugin for ExporterPlugin {
    fn manifest(&self) -> PluginManifest {
        PluginManifest::new(
            format!("exporter-{}", self.format),
            self.format,
            "0.1.0",
            PluginCategory::Exporter,
        )
    }

    async fn apply(&self, ctx: &Context) -> CoreResult<()> {
        self.applied.lock().unwrap().push(self.format.to_string());
        let exporter: Arc<dyn Exporter> = Arc::new(NamedExporter(self.format));
        ctx.provide_named(self.format, exporter).await
    }
}

async fn setup(formats: &[&'static str]) -> (Context, PluginManager, Vec<String>) {
    let ctx = Context::new("registry_test");
    let manager = PluginManager::new();
    let applied = Arc::new(Mutex::new(Vec::new()));
    for format in formats {
        manager
            .register(Arc::new(ExporterPlugin {
                format,
                applied: applied.clone(),
            }))
            .await
            .unwrap();
    }
    manager.apply_all(&ctx).await.unwrap();
    let order = applied.lock().unwrap().clone();
    (ctx, manager, order)
}

#[tokio::test]
async fn same_type_providers_coexist_and_resolve_by_name() {
    let (ctx, _, _) = setup(&["csv", "json"]).await;

    let csv = ctx.inject_named::<Arc<dyn Exporter>>("csv").await.unwrap();
    let json = ctx.inject_named::<Arc<dyn Exporter>>("json").await.unwrap();
    assert_eq!(csv.format_name(), "csv");
    assert_eq!(json.format_name(), "json");

    let all: Vec<String> = ctx
        .inject_all::<Arc<dyn Exporter>>()
        .await
        .into_iter()
        .map(|(name, _)| name)
        .collect();
    assert_eq!(all, vec!["csv", "json"]);

    assert!(matches!(
        ctx.inject_named::<Arc<dyn Exporter>>("xml").await,
        Err(CoreError::ServiceNotFound(_))
    ));
}

#[tokio::test]
async fn default_provider_is_deterministic_and_overridable() {
    for _ in 0..30 {
        let (ctx, manager, applied) = setup(&["json", "csv", "md"]).await;
        assert_eq!(applied, vec!["json", "csv", "md"]);
        let ids: Vec<String> = manager.list_manifests().await.into_iter().map(|m| m.id).collect();
        assert_eq!(ids, vec!["exporter-json", "exporter-csv", "exporter-md"]);
        let default = ctx.inject::<Arc<dyn Exporter>>().await.unwrap();
        assert_eq!(default.format_name(), "json");
    }

    let (ctx, _, _) = setup(&["json", "csv"]).await;
    ctx.set_default::<Arc<dyn Exporter>>("csv").await.unwrap();
    assert_eq!(ctx.inject::<Arc<dyn Exporter>>().await.unwrap().format_name(), "csv");
    assert!(ctx.set_default::<Arc<dyn Exporter>>("xml").await.is_err());
}

#[tokio::test]
async fn duplicate_service_name_is_an_explicit_conflict() {
    let ctx = Context::new("conflict");
    let first: Arc<dyn Exporter> = Arc::new(NamedExporter("csv"));
    let second: Arc<dyn Exporter> = Arc::new(NamedExporter("csv-v2"));
    ctx.provide_named("csv", first).await.unwrap();

    let err = ctx.provide_named("csv", second).await.unwrap_err();
    assert!(matches!(err, CoreError::ServiceConflict { ref name, .. } if name == "csv"));
    let kept = ctx.inject_named::<Arc<dyn Exporter>>("csv").await.unwrap();
    assert_eq!(kept.format_name(), "csv");

    ctx.provide(String::from("a")).await.unwrap();
    assert!(ctx.provide(String::from("b")).await.is_err());
    assert_eq!(ctx.inject::<String>().await.unwrap().as_str(), "a");
}

#[tokio::test]
async fn duplicate_plugin_id_is_rejected() {
    let manager = PluginManager::new();
    let applied = Arc::new(Mutex::new(Vec::new()));
    let plugin = || {
        Arc::new(ExporterPlugin {
            format: "csv",
            applied: applied.clone(),
        })
    };
    manager.register(plugin()).await.unwrap();
    let err = manager.register(plugin()).await.unwrap_err();
    assert!(matches!(err, CoreError::PluginConflict(ref id) if id == "exporter-csv"));
    assert_eq!(manager.list_manifests().await.len(), 1);
}
