use std::sync::Arc;

use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::http::{HeaderName, Method};
use axum::middleware;
use tokio::sync::Mutex;
use tower_http::cors::{AllowOrigin, CorsLayer};

use crate::config::Settings;
use crate::middleware::request_id_middleware;
use crate::modules::{system, v2};
use crate::state::AppState;
use crate::storage::store::Store;

pub async fn build_app_state(settings: Arc<Settings>, store: Store) -> AppState {
    let context = leo_core::Context::new("leo_app");
    let plugin_manager = Arc::new(leo_core::PluginManager::new());

    // 注册并激活官方内置插件
    if let Err(err) =
        crate::plugins::setup_default_plugins(&context, &plugin_manager, &settings.csv_dir).await
    {
        tracing::warn!("装配官方插件时出现警告: {err}");
    }

    AppState {
        settings,
        store: Arc::new(Mutex::new(store)),
        context,
        plugin_manager,
    }
}

pub fn create_cors_layer(settings: &Settings) -> CorsLayer {
    if settings.cors_origins.iter().any(|origin| origin == "*") {
        CorsLayer::new()
            .allow_origin(AllowOrigin::mirror_request())
            .allow_methods([
                Method::GET,
                Method::POST,
                Method::PUT,
                Method::PATCH,
                Method::DELETE,
                Method::OPTIONS,
            ])
            .allow_headers([
                HeaderName::from_static("content-type"),
                HeaderName::from_static("x-request-id"),
            ])
            .allow_credentials(true)
    } else {
        CorsLayer::new()
            .allow_origin(AllowOrigin::list(
                settings
                    .cors_origins
                    .iter()
                    .filter_map(|origin| origin.parse().ok()),
            ))
            .allow_methods([
                Method::GET,
                Method::POST,
                Method::PUT,
                Method::PATCH,
                Method::DELETE,
                Method::OPTIONS,
            ])
            .allow_headers([
                HeaderName::from_static("content-type"),
                HeaderName::from_static("x-request-id"),
            ])
            .allow_credentials(true)
    }
}

pub async fn build_router(settings: Arc<Settings>, store: Store) -> Router {
    let state = build_app_state(settings, store).await;
    build_router_from_state(state)
}

pub fn build_router_from_state(state: AppState) -> Router {
    let cors = create_cors_layer(&state.settings);
    let max_upload = state
        .settings
        .max_upload_size_mb
        .saturating_mul(1024 * 1024)
        + 2 * 1024 * 1024;

    let api = Router::new()
        .nest(
            "/api/v2",
            v2::router().merge(crate::modules::audio::router()),
        )
        .merge(system::root_router())
        .layer(middleware::from_fn(request_id_middleware))
        .with_state(state);

    api.layer(cors).layer(DefaultBodyLimit::max(max_upload))
}
