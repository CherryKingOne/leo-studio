pub mod app;
pub mod config;
pub mod error;
pub mod http;
pub mod middleware;
pub mod modules;
pub mod response;
pub mod state;
pub mod storage;
pub mod text;

use std::sync::Arc;

use async_trait::async_trait;
use leo_core::context::Context;
use leo_core::error::CoreResult;
use leo_core::plugin::{Plugin, PluginCategory, PluginManifest};

pub use app::{
    build_app_state, build_app_state_with, build_router, build_router_from_state, create_cors_layer,
};
pub use config::Settings;
pub use error::{AppError, AppResult, ErrorCode};
pub use response::ApiResponse;
pub use state::AppState;
pub use storage::store::Store;

/// Leo Studio RESTful API 服务插件
pub struct ApiPlugin {
    pub settings: Arc<Settings>,
}

impl ApiPlugin {
    pub fn new(settings: Arc<Settings>) -> Self {
        Self { settings }
    }
}

#[async_trait]
impl Plugin for ApiPlugin {
    fn manifest(&self) -> PluginManifest {
        let mut manifest = PluginManifest::new(
            "leo-plugin-api",
            "Leo Studio REST API 传输服务插件",
            "0.2.0",
            PluginCategory::Api,
        );
        manifest.description = "提供基于 Axum 的 v3 项目管理 HTTP 接口。".into();
        manifest.capabilities = vec![
            "transport:http".into(),
            "api:projects".into(),
            "api:characters".into(),
            "api:dialogues".into(),
            "api:v3".into(),
        ];
        manifest
    }

    async fn apply(&self, _ctx: &Context) -> CoreResult<()> {
        tracing::info!(
            "已挂载 ApiPlugin: 预定服务端口为 {}:{}",
            self.settings.host,
            self.settings.port
        );
        Ok(())
    }
}
