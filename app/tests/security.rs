mod common;

use axum::body::Body;
use axum::http::{Request, Response};
use common::TestApp;
use leo_app::app::origin_matches;
use leo_app::config::{Settings, default_cors_origins};
use tower::ServiceExt;

async fn call(app: &TestApp, request: Request<Body>) -> Response<Body> {
    app.router.clone().oneshot(request).await.unwrap()
}

fn get_with_origin(path: &str, origin: &str) -> Request<Body> {
    Request::builder()
        .uri(path)
        .header("origin", origin)
        .body(Body::empty())
        .unwrap()
}

fn header<'a>(response: &'a Response<Body>, name: &str) -> Option<&'a str> {
    response.headers().get(name).and_then(|v| v.to_str().ok())
}

#[test]
fn defaults_bind_loopback_and_localhost_cors() {
    if std::env::var("LEO_HOST").is_err() {
        assert_eq!(Settings::from_env().host, "127.0.0.1");
    }
    if std::env::var("LEO_CORS_ORIGINS").is_err() {
        assert_eq!(Settings::from_env().cors_origins, default_cors_origins());
    }
    if std::env::var("LEO_API_TOKEN").is_err() {
        assert!(Settings::from_env().api_token.is_none());
    }

    assert!(origin_matches("http://localhost:*", "http://localhost:5173"));
    assert!(origin_matches("http://localhost:*", "http://localhost"));
    assert!(!origin_matches("http://localhost:*", "http://localhost.evil.example"));
    assert!(!origin_matches("http://localhost:*", "http://localhost:5173.evil.example"));
    assert!(!origin_matches("http://localhost:*", "https://localhost:5173"));
    assert!(origin_matches("https://studio.example", "https://studio.example"));
    assert!(!origin_matches("https://studio.example", "https://studio.example.evil"));
}

#[tokio::test]
async fn cors_default_allowlist_rejects_foreign_origin() {
    let dir = tempfile::tempdir().unwrap();
    let app = TestApp::with_config(dir.path(), 1, |s| s.cors_origins = default_cors_origins()).await;

    let response = call(&app, get_with_origin("/api/v2/health", "https://evil.example")).await;
    assert_eq!(header(&response, "access-control-allow-origin"), None);

    let response = call(&app, get_with_origin("/api/v2/health", "http://localhost:5173")).await;
    assert_eq!(
        header(&response, "access-control-allow-origin"),
        Some("http://localhost:5173")
    );
    assert_eq!(header(&response, "access-control-allow-credentials"), Some("true"));
}

#[tokio::test]
async fn cors_wildcard_never_sends_credentials() {
    let dir = tempfile::tempdir().unwrap();
    let app = TestApp::with_config(dir.path(), 1, |s| s.cors_origins = vec!["*".into()]).await;

    let response = call(&app, get_with_origin("/api/v2/health", "https://evil.example")).await;
    assert_eq!(header(&response, "access-control-allow-origin"), Some("*"));
    assert_eq!(header(&response, "access-control-allow-credentials"), None);
}

#[tokio::test]
async fn api_token_is_enforced_when_configured() {
    let dir = tempfile::tempdir().unwrap();
    let app = TestApp::with_config(dir.path(), 1, |s| s.api_token = Some("s3cret".into())).await;

    let (status, _) = app.get("/api/v2/health").await;
    assert_eq!(status, 200);

    let (status, body) = app.get("/api/v2/projects").await;
    assert_eq!(status, 401);
    assert_eq!(body["code"], 40100);

    let (status, _) = app
        .send(
            Request::builder()
                .uri("/api/v2/projects")
                .header("authorization", "Bearer wrong")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(status, 401);

    let (status, _) = app
        .send(
            Request::builder()
                .uri("/api/v2/projects")
                .header("authorization", "Bearer s3cret")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(status, 200);

    // 预检请求不需要 token
    let response = call(
        &app,
        Request::builder()
            .method("OPTIONS")
            .uri("/api/v2/projects")
            .header("origin", "http://localhost:5173")
            .header("access-control-request-method", "GET")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert!(response.status().is_success());
}

#[tokio::test]
async fn api_token_bearer_scheme_is_case_insensitive() {
    let dir = tempfile::tempdir().unwrap();
    let app = TestApp::with_config(dir.path(), 1, |s| s.api_token = Some("s3cret".into())).await;

    let (status, _) = app
        .send(
            Request::builder()
                .uri("/api/v2/projects")
                .header("authorization", "bearer s3cret")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(status, 200);

    let (status, _) = app
        .send(
            Request::builder()
                .uri("/api/v2/projects")
                .header("authorization", "Basic s3cret")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(status, 401);
}

#[tokio::test]
async fn api_token_absent_means_open_access() {
    let dir = tempfile::tempdir().unwrap();
    let app = TestApp::with_config(dir.path(), 1, |s| s.api_token = None).await;
    let (status, _) = app.get("/api/v2/projects").await;
    assert_eq!(status, 200);
}

#[tokio::test]
async fn mcp_endpoint_requires_token_when_configured() {
    use std::sync::Arc;

    use leo_app::mcp::{McpSettings, http_router};
    use leo_app::storage::store::Store;
    use leo_app::{build_app_state, require_api_token};

    let dir = tempfile::tempdir().unwrap();
    let settings = Arc::new(Settings {
        data_dir: dir.path().to_path_buf(),
        upload_dir: dir.path().join("uploads"),
        csv_dir: dir.path().join("csv"),
        api_token: Some("s3cret".into()),
        ..Settings::default()
    });
    let store = Store::new(&settings).unwrap();
    let state = build_app_state(settings.clone(), store).await.unwrap();
    let mcp = Arc::new(McpSettings::default());
    let router = axum::Router::new().nest(
        "/mcp",
        http_router(mcp, state).layer(axum::middleware::from_fn_with_state(
            settings,
            require_api_token,
        )),
    );

    let request = || {
        Request::builder()
            .method("POST")
            .uri("/mcp")
            .header("content-type", "application/json")
            .header("accept", "application/json, text/event-stream")
            .body(Body::from(
                r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"t","version":"0"}}}"#,
            ))
            .unwrap()
    };

    let response = router.clone().oneshot(request()).await.unwrap();
    assert_eq!(response.status(), 401);

    let mut authed = request();
    authed
        .headers_mut()
        .insert("authorization", "Bearer s3cret".parse().unwrap());
    let response = router.oneshot(authed).await.unwrap();
    assert_ne!(response.status(), 401);
}
