use std::sync::Arc;

use leo_plugin_api::{
    ApiPlugin, Settings, Store, build_app_state, build_router_from_state, require_api_token,
};
use leo_plugin_mcp::{McpPlugin, McpSettings, http_router};

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

    let state = build_app_state(settings.clone(), store).await;

    // 挂载 API 与 MCP 插件清单
    let api_plugin = Arc::new(ApiPlugin::new(settings.clone()));
    state.plugin_manager.register(api_plugin).await.expect("注册 ApiPlugin 失败");

    let mcp_settings = Arc::new(McpSettings::from_env());
    let mcp_plugin = Arc::new(McpPlugin::new(mcp_settings.clone()));
    state.plugin_manager.register(mcp_plugin).await.expect("注册 McpPlugin 失败");

    let mut router = build_router_from_state(state.clone());
    if mcp_settings.enabled {
        let path = mcp_settings.http_path.clone();
        tracing::info!("挂载 MCP HTTP 服务端点: {path}");
        let mcp = http_router(mcp_settings, state).layer(axum::middleware::from_fn_with_state(
            settings.clone(),
            require_api_token,
        ));
        router = router.nest(&path, mcp);
    }

    if settings.api_token.is_none() && !is_loopback_host(&settings.host) {
        tracing::warn!("服务监听在非本机地址 {address} 且未配置 LEO_API_TOKEN，局域网内任何人都可调用");
    }

    tracing::info!("Leo Studio 后端微内核已启动：http://{address}");

    if let Err(err) = axum::serve(listener, router).await {
        tracing::error!("服务异常退出：{err}");
        std::process::exit(1);
    }
}

fn is_loopback_host(host: &str) -> bool {
    host == "localhost"
        || host
            .trim_matches(|c| c == '[' || c == ']')
            .parse::<std::net::IpAddr>()
            .is_ok_and(|ip| ip.is_loopback())
}
