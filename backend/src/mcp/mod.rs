pub mod config;
pub mod prompts;
pub mod registry;
pub mod resources;
pub mod server;

pub use config::McpSettings;
pub use server::McpHandler;

use std::sync::Arc;

use axum::{
    Router,
    extract::State,
    http::{Request, StatusCode, header::AUTHORIZATION},
    middleware::Next,
    response::{IntoResponse, Response},
};
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
};

use crate::state::AppState;

/// 构建挂载在 Axum 上的 Streamable HTTP 入口。
pub fn http_router(mcp: Arc<McpSettings>, state: AppState) -> Router {
    let handler = McpHandler::new(state.settings.clone(), state.store.clone(), mcp.clone());

    let config = StreamableHttpServerConfig::default()
        .with_legacy_session_mode(false)
        .with_json_response(true)
        .with_stateless_protocol_metadata_required(true);

    let service = StreamableHttpService::new(
        move || Ok(handler.clone()),
        LocalSessionManager::default().into(),
        config,
    );

    let app: Router<Arc<McpSettings>> = Router::new().fallback_service(service);
    let app = if mcp.token.is_empty() {
        app
    } else {
        app.layer(axum::middleware::from_fn_with_state(
            mcp.clone(),
            require_bearer,
        ))
    };

    app.with_state(mcp)
}

async fn require_bearer(
    State(mcp): State<Arc<McpSettings>>,
    request: Request<axum::body::Body>,
    next: Next,
) -> Response {
    let expected = format!("Bearer {}", mcp.token);
    let authorized = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value == expected);

    if authorized {
        next.run(request).await
    } else {
        (
            StatusCode::UNAUTHORIZED,
            "缺少或无效的 Authorization: Bearer <token>",
        )
            .into_response()
    }
}
