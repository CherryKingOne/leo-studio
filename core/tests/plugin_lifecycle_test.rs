use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use leo_core::context::Context;
use leo_core::error::{CoreError, CoreResult};
use leo_core::plugin::{FailurePolicy, Plugin, PluginCategory, PluginManager, PluginManifest};

type Journal = Arc<Mutex<Vec<String>>>;

struct TestPlugin {
    id: &'static str,
    deps: Vec<&'static str>,
    fail: bool,
    journal: Journal,
}

impl TestPlugin {
    fn new(id: &'static str, deps: &[&'static str], journal: &Journal) -> Arc<Self> {
        Arc::new(Self {
            id,
            deps: deps.to_vec(),
            fail: false,
            journal: journal.clone(),
        })
    }

    fn failing(id: &'static str, journal: &Journal) -> Arc<Self> {
        Arc::new(Self {
            id,
            deps: Vec::new(),
            fail: true,
            journal: journal.clone(),
        })
    }
}

#[async_trait]
impl Plugin for TestPlugin {
    fn manifest(&self) -> PluginManifest {
        let mut manifest = PluginManifest::new(self.id, self.id, "0.1.0", PluginCategory::Other);
        manifest.dependencies = self.deps.iter().map(|d| d.to_string()).collect();
        manifest
    }

    async fn apply(&self, ctx: &Context) -> CoreResult<()> {
        ctx.provide_named(self.id, self.id.to_string()).await?;
        if self.fail {
            return Err(CoreError::Internal(format!("{} 初始化失败", self.id)));
        }
        self.journal.lock().unwrap().push(format!("apply:{}", self.id));
        Ok(())
    }

    async fn dispose(&self, _ctx: &Context) -> CoreResult<()> {
        self.journal.lock().unwrap().push(format!("dispose:{}", self.id));
        Ok(())
    }
}

fn entries(journal: &Journal) -> Vec<String> {
    journal.lock().unwrap().clone()
}

async fn service_names(ctx: &Context) -> Vec<String> {
    ctx.inject_all::<String>().await.into_iter().map(|(name, _)| name).collect()
}

#[tokio::test]
async fn plugins_are_applied_in_dependency_order() {
    let journal = Journal::default();
    let ctx = Context::new("deps");
    let manager = PluginManager::new();
    manager.register(TestPlugin::new("mcp", &["api"], &journal)).await.unwrap();
    manager.register(TestPlugin::new("api", &["storage"], &journal)).await.unwrap();
    manager.register(TestPlugin::new("storage", &[], &journal)).await.unwrap();
    manager.register(TestPlugin::new("tts", &[], &journal)).await.unwrap();

    manager.apply_all(&ctx).await.unwrap();

    assert_eq!(
        entries(&journal),
        vec!["apply:storage", "apply:api", "apply:mcp", "apply:tts"]
    );
    assert_eq!(manager.loaded_ids().await, vec!["storage", "api", "mcp", "tts"]);

    manager.apply_all(&ctx).await.unwrap();
    assert_eq!(entries(&journal).len(), 4, "已挂载的插件不会被重复挂载");
}

#[tokio::test]
async fn missing_dependency_fails_before_anything_is_applied() {
    let journal = Journal::default();
    let ctx = Context::new("missing");
    let manager = PluginManager::new();
    manager.register(TestPlugin::new("storage", &[], &journal)).await.unwrap();
    manager.register(TestPlugin::new("mcp", &["api"], &journal)).await.unwrap();

    let err = manager.apply_all(&ctx).await.unwrap_err();
    assert!(matches!(
        err,
        CoreError::DependencyMissing { ref plugin_id, ref dependency }
            if plugin_id == "mcp" && dependency == "api"
    ));
    assert!(entries(&journal).is_empty());
    assert!(manager.loaded_ids().await.is_empty());
}

#[tokio::test]
async fn dependency_cycle_is_reported() {
    let journal = Journal::default();
    let manager = PluginManager::new();
    manager.register(TestPlugin::new("a", &["b"], &journal)).await.unwrap();
    manager.register(TestPlugin::new("b", &["a"], &journal)).await.unwrap();

    let err = manager.apply_all(&Context::new("cycle")).await.unwrap_err();
    assert!(matches!(err, CoreError::DependencyCycle(_)));
    assert!(entries(&journal).is_empty());
}

#[tokio::test]
async fn fail_fast_rolls_back_already_applied_plugins() {
    let journal = Journal::default();
    let ctx = Context::new("fail_fast");
    let manager = PluginManager::new();
    manager.register(TestPlugin::new("storage", &[], &journal)).await.unwrap();
    manager.register(TestPlugin::failing("broken", &journal)).await.unwrap();
    manager.register(TestPlugin::new("tts", &[], &journal)).await.unwrap();

    let err = manager.apply_all(&ctx).await.unwrap_err();
    assert!(matches!(err, CoreError::Plugin { ref plugin_id, .. } if plugin_id == "broken"));
    assert_eq!(entries(&journal), vec!["apply:storage", "dispose:storage"]);
    assert!(manager.loaded_ids().await.is_empty());
    assert!(service_names(&ctx).await.is_empty(), "失败与回滚插件注册的服务均已撤销");
}

#[tokio::test]
async fn skip_failed_policy_skips_failed_plugin_and_its_dependents() {
    let journal = Journal::default();
    let ctx = Context::new("skip");
    let manager = PluginManager::with_failure_policy(FailurePolicy::SkipFailed);
    manager.register(TestPlugin::failing("storage", &journal)).await.unwrap();
    manager.register(TestPlugin::new("api", &["storage"], &journal)).await.unwrap();
    manager.register(TestPlugin::new("tts", &[], &journal)).await.unwrap();
    manager.register(TestPlugin::new("orphan", &["nowhere"], &journal)).await.unwrap();

    manager.apply_all(&ctx).await.unwrap();

    assert_eq!(manager.loaded_ids().await, vec!["tts"]);
    assert_eq!(service_names(&ctx).await, vec!["tts"]);
    let mut failed: Vec<String> = manager
        .failures()
        .await
        .into_iter()
        .map(|f| f.plugin_id)
        .collect();
    failed.sort();
    assert_eq!(failed, vec!["api", "orphan", "storage"]);
}

#[tokio::test]
async fn unload_disposes_plugin_and_revokes_its_services() {
    let journal = Journal::default();
    let ctx = Context::new("unload");
    let manager = PluginManager::new();
    manager.register(TestPlugin::new("storage", &[], &journal)).await.unwrap();
    manager.register(TestPlugin::new("api", &["storage"], &journal)).await.unwrap();
    manager.register(TestPlugin::new("tts", &[], &journal)).await.unwrap();
    manager.apply_all(&ctx).await.unwrap();

    let err = manager.unload("storage", &ctx).await.unwrap_err();
    assert!(matches!(err, CoreError::Plugin { ref message, .. } if message.contains("api")));

    manager.unload("tts", &ctx).await.unwrap();
    assert!(ctx.inject_named::<String>("tts").await.is_err());
    assert_eq!(manager.loaded_ids().await, vec!["storage", "api"]);
    assert!(matches!(
        manager.unload("tts", &ctx).await,
        Err(CoreError::PluginNotFound(_))
    ));

    manager.dispose_all(&ctx).await.unwrap();
    assert!(manager.loaded_ids().await.is_empty());
    assert!(service_names(&ctx).await.is_empty());
    assert_eq!(
        &entries(&journal)[3..],
        &["dispose:tts", "dispose:api", "dispose:storage"]
    );

    manager.apply_all(&ctx).await.unwrap();
    assert_eq!(service_names(&ctx).await, vec!["storage", "api", "tts"]);
}
