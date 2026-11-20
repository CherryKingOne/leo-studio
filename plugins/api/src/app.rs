use std::sync::Arc;

use axum::Router;
use axum::extract::{DefaultBodyLimit, Request, State};
use axum::http::header::AUTHORIZATION;
use axum::http::{HeaderName, HeaderValue, Method};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use tokio::sync::Mutex;
use tower_http::cors::{AllowOrigin, CorsLayer};

use crate::config::Settings;
use crate::error::AppError;
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
    let layer = CorsLayer::new()
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
            HeaderName::from_static("authorization"),
        ]);

    if settings.cors_origins.iter().any(|origin| origin == "*") {
        // 通配来源不允许携带凭据，避免任意网站以用户身份调用
        tracing::warn!("LEO_CORS_ORIGINS=* 已允许任意来源跨域访问（不携带凭据）");
        return layer.allow_origin(AllowOrigin::any());
    }

    let patterns = settings.cors_origins.clone();
    layer
        .allow_origin(AllowOrigin::predicate(move |origin: &HeaderValue, _| {
            origin
                .to_str()
                .map(|origin| patterns.iter().any(|pattern| origin_matches(pattern, origin)))
                .unwrap_or(false)
        }))
        .allow_credentials(true)
}

/// 精确匹配来源；`scheme://host:*` 额外匹配该主机的任意端口（含省略端口）。
pub fn origin_matches(pattern: &str, origin: &str) -> bool {
    let pattern = pattern.trim().trim_end_matches('/');
    match pattern.strip_suffix(":*") {
        Some(prefix) => {
            origin == prefix
                || origin
                    .strip_prefix(prefix)
                    .and_then(|rest| rest.strip_prefix(':'))
                    .is_some_and(|port| !port.is_empty() && port.chars().all(|c| c.is_ascii_digit()))
        }
        None => origin == pattern,
    }
}

/// 配置了 `LEO_API_TOKEN` 时，要求请求携带 `Authorization: Bearer <token>`；健康检查除外。
pub async fn require_api_token(
    State(settings): State<Arc<Settings>>,
    request: Request,
    next: Next,
) -> Response {
    let Some(expected) = settings.api_token.as_deref() else {
        return next.run(request).await;
    };
    let path = request.uri().path();
    if request.method() == Method::OPTIONS || matches!(path, "/health" | "/api/v2/health") {
        return next.run(request).await;
    }

    let provided = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split_once(' '))
        .filter(|(scheme, _)| scheme.eq_ignore_ascii_case("bearer"))
        .map(|(_, token)| token.trim());

    match provided {
        Some(token) if constant_time_eq(token.as_bytes(), expected.as_bytes()) => {
            next.run(request).await
        }
        _ => AppError::unauthorized("缺少或无效的 API Token").into_response(),
    }
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
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
        .layer(middleware::from_fn_with_state(
            state.settings.clone(),
            require_api_token,
        ))
        .layer(middleware::from_fn(request_id_middleware))
        .with_state(state);

    api.layer(cors).layer(DefaultBodyLimit::max(max_upload))
}
