mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::TestApp;

#[tokio::test]
async fn removed_system_audio_exchange_routes_return_404() {
    let app = TestApp::new(1).await;

    for path in [
        "/",
        "/health",
        "/system/health",
        "/api/v2/",
        "/api/v2/health",
        "/api/v2/voices",
        "/api/v2/plugins",
        "/api/v2/audio/missing",
        "/api/v2/export/missing",
        "/api/v1/health",
        "/api/v1/voices",
        "/api/v1/export/missing",
    ] {
        let (status, _) = app.get(path).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "path={path}");
    }

    let (status, _) = app
        .send(
            Request::builder()
                .method("POST")
                .uri("/api/v2/projects/p/chapters/c/synthesize")
                .header("content-type", "application/json")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let (status, _) = app
        .post_multipart(
            "/api/v2/import",
            &[],
            Some(("novel.txt", "text/plain", b"chapter".as_slice())),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
