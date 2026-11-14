pub mod config;
pub mod prompts;
pub mod registry;
pub mod resources;
pub mod server;

pub use config::McpSettings;
pub use server::McpHandler;

use std::sync::Arc;

use axum::Router;
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
};

use crate::state::AppState;

/// 构建挂载在 Axum 上的 Streamable HTTP 入口。
pub fn http_router(mcp: Arc<McpSettings>, state: AppState) -> Router {
    let handler = McpHandler::new(state.settings.clone(), state.store.clone(), mcp.clone());

    let config = StreamableHttpServerConfig::default()
        .with_legacy_session_mode(true)
        .with_json_response(true)
        .with_stateless_protocol_metadata_required(true);

    let service = StreamableHttpService::new(
        move || Ok(handler.clone()),
        LocalSessionManager::default().into(),
        config,
    );

    Router::new().fallback_service(service)
}
