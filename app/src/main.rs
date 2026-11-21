use std::sync::Arc;

use leo_app::runtime::build_state;
use leo_plugin_api::{Settings, Store, build_router_from_state};
use leo_plugin_mcp::{McpSettings, http_router};

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let settings = Arc::new(Settings::from_env());
    let store = match Store::new(&settings) {
        Ok(store) => store,
        Err(err) => {
            tracing::error!("初始化数据存储失败：{err}");
            std::process::exit(1);
        }
    };

    let address = format!("{}:{}", settings.host, settings.port);
    let listener = match tokio::net::TcpListener::bind(&address).await {
        Ok(listener) => listener,
        Err(err) => {
            tracing::error!("监听 {address} 失败：{err}");
            std::process::exit(1);
        }
    };

    // 插件：目录里的插件（按 leo-plugins.json / LEO_PLUGINS_CONFIG 启用）+ API / MCP 接口插件
    let mcp_settings = Arc::new(McpSettings::from_env());
    let state = match build_state(settings.clone(), store, mcp_settings.clone(), None).await {
        Ok(state) => state,
        Err(err) => {
            tracing::error!("装配插件失败：{err}");
            std::process::exit(1);
        }
    };

    let mut router = build_router_from_state(state.clone());
    if mcp_settings.enabled {
        let path = mcp_settings.http_path.clone();
        tracing::info!("挂载 MCP HTTP 服务端点: {path}");
        router = router.nest(&path, http_router(mcp_settings, state));
    }

    tracing::info!("Leo Studio 后端微内核已启动：http://{address}");

    if let Err(err) = axum::serve(listener, router).await {
        tracing::error!("服务异常退出：{err}");
        std::process::exit(1);
    }
}
