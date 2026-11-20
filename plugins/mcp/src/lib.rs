pub mod config;
pub mod prompts;
pub mod registry;
pub mod resources;
pub mod server;

pub use config::McpSettings;
pub use server::McpHandler;

use std::sync::Arc;

use async_trait::async_trait;
use axum::Router;
use leo_core::context::Context;
use leo_core::error::CoreResult;
use leo_core::plugin::{Plugin, PluginCategory, PluginManifest};
use leo_plugin_api::state::AppState;
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
};

/// Leo Studio MCP 协议服务插件
pub struct McpPlugin {
    pub settings: Arc<McpSettings>,
}

impl McpPlugin {
    pub fn new(settings: Arc<McpSettings>) -> Self {
        Self { settings }
    }
}

#[async_trait]
impl Plugin for McpPlugin {
    fn manifest(&self) -> PluginManifest {
        let mut manifest = PluginManifest::new(
            "leo-plugin-mcp",
            "Model Context Protocol 服务插件",
            "0.2.0",
            PluginCategory::Mcp,
        );
        manifest.description =
            "提供基于 MCP 标准的 AI Agent 工具调用、剧本文档资源与工作流 Prompt 提示词集成。".into();
        manifest.capabilities = vec![
            "mcp:tools".into(),
            "mcp:resources".into(),
            "mcp:prompts".into(),
            "mcp:streamable-http".into(),
        ];
        manifest.dependencies = vec!["leo-plugin-api".into()];
        manifest
    }

    async fn apply(&self, _ctx: &Context) -> CoreResult<()> {
        tracing::info!(
            "已挂载 McpPlugin: 启用状态 = {}, HTTP 挂载路径 = {}",
            self.settings.enabled,
            self.settings.http_path
        );
        Ok(())
    }
}

/// 构建挂载在 Axum 上的 Streamable HTTP 入口。
pub fn http_router(mcp: Arc<McpSettings>, state: AppState) -> Router {
    let handler = McpHandler::new(state, mcp.clone());

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
