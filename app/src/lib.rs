pub use leo_plugin_api::*;
pub use leo_plugin_mcp as mcp;

/// 宿主装配：插件目录（leo-plugin-catalog）+ API / MCP 接口插件
pub mod runtime {
    use std::path::Path;
    use std::sync::Arc;

    use axum::Router;
    use leo_core::error::CoreResult;
    use leo_core::{Context, PluginManager};
    use leo_plugin_api::{ApiPlugin, AppState, Settings, Store, build_app_state_with, build_router_from_state};
    use leo_plugin_catalog::{HostEnv, PluginsConfig};
    use leo_plugin_mcp::{McpPlugin, McpSettings};

    /// 装配全部插件并构建 AppState。
    ///
    /// API / MCP 接口插件与目录里的插件一起登记、一起装配（之前它们在 apply_all 之后才登记，apply 从未执行）。
    pub async fn build_state(
        settings: Arc<Settings>,
        store: Store,
        mcp: Arc<McpSettings>,
        plugins_config: Option<&Path>,
    ) -> CoreResult<AppState> {
        let config = PluginsConfig::load(plugins_config)?;
        let env = HostEnv { csv_dir: settings.csv_dir.clone() };
        let context = Context::new("leo_app");
        let manager = Arc::new(PluginManager::new());
        manager.register(Arc::new(ApiPlugin::new(settings.clone()))).await?;
        manager.register(Arc::new(McpPlugin::new(mcp))).await?;
        leo_plugin_catalog::install(&context, &manager, &env, &config).await?;
        Ok(build_app_state_with(settings, store, context, manager))
    }

    /// 测试 / 嵌入用：用默认插件配置构建路由（不挂 MCP HTTP）
    pub async fn build_router(settings: Arc<Settings>, store: Store) -> CoreResult<Router> {
        let mcp = Arc::new(McpSettings::from_env());
        let state = build_state(settings, store, mcp, None).await?;
        Ok(build_router_from_state(state))
    }
}
