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

/// 用一个空的插件上下文构建状态（不装配任何插件；宿主程序应使用 [`build_app_state_with`]）
pub async fn build_app_state(settings: Arc<Settings>, store: Store) -> AppState {
    build_app_state_with(
        settings,
        store,
        leo_core::Context::new("leo_app"),
        Arc::new(leo_core::PluginManager::new()),
    )
}

/// 用宿主装配好的插件上下文构建状态。
///
/// API 插件不再自己装配具体插件：有哪些插件由宿主（app / cli）通过 leo-plugin-catalog 决定，
/// API 只通过各注册层（TextHub / ExchangeHub / StorageHub / TtsHub）使用它们。
pub fn build_app_state_with(
    settings: Arc<Settings>,
    store: Store,
    context: leo_core::Context,
    plugin_manager: Arc<leo_core::PluginManager>,
) -> AppState {
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
