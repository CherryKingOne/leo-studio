mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::TestApp;

const SAMPLE_NOVEL: &str =
    "第1章 绯红之月\n痛！好痛！头好痛！\n\n第2章 宿命之枪\n左轮手枪与桌上的日记。";

#[tokio::test]
async fn v3_recycle_bin_restore_and_hard_delete() {
    let app = TestApp::new(1).await;

    let (_, body) = app
        .post_multipart(
            "/api/v3/projects",
            &[("name", "回收项目")],
            Some(("novel.txt", "text/plain", SAMPLE_NOVEL.as_bytes())),
        )
        .await;
    let project_id = body["data"]["project_id"].as_str().unwrap().to_string();

    let (status, _) = app.delete(&format!("/api/v3/projects/{project_id}")).await;
    assert_eq!(status, StatusCode::OK);

    let (status, body) = app.get("/api/v3/recycle-bin").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["total"], 1);

    let (status, _) = app
        .send(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v3/recycle-bin/{project_id}/restore"))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    let (status, body) = app
        .delete(&format!("/api/v3/recycle-bin/{project_id}"))
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["code"], 40000);

    let (status, _) = app.delete(&format!("/api/v3/projects/{project_id}")).await;
    assert_eq!(status, StatusCode::OK);

    let (status, body) = app
        .delete(&format!("/api/v3/recycle-bin/{project_id}"))
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["hard_deleted"], true);

    let (status, body) = app.get(&format!("/api/v3/projects/{project_id}")).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["code"], 40400);
}

#[tokio::test]
async fn v3_recycle_bin_clear_and_pagination_validation() {
    let app = TestApp::new(1).await;

    for name in ["清空项目一", "清空项目二"] {
        let (_, body) = app
            .post_multipart(
                "/api/v3/projects",
                &[("name", name)],
                Some(("novel.txt", "text/plain", SAMPLE_NOVEL.as_bytes())),
            )
            .await;
        let project_id = body["data"]["project_id"].as_str().unwrap().to_string();
        let (status, _) = app.delete(&format!("/api/v3/projects/{project_id}")).await;
        assert_eq!(status, StatusCode::OK);
    }

    let (status, body) = app.delete("/api/v3/recycle-bin").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["cleared_count"], 2);
    assert_eq!(body["data"]["cleared"], true);

    let (status, body) = app.get("/api/v3/recycle-bin").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["total"], 0);

    let (status, body) = app.get("/api/v3/recycle-bin?page=0").await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["code"], 40000);
}

#[tokio::test]
async fn legacy_recycle_bin_routes_are_removed() {
    let app = TestApp::new(1).await;

    let (status, _) = app.get("/api/v2/projects/recycle-bin").await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let (status, _) = app.delete("/api/v2/projects/recycle-bin").await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    for path in [
        "/api/v2/projects/missing/restore",
        "/api/v2/projects/missing/hard",
        "/api/v1/projects/missing/restore",
        "/api/v1/projects/missing/hard",
    ] {
        let method = if path.ends_with("/restore") {
            "POST"
        } else {
            "DELETE"
        };
        let (status, _) = app
            .send(
                Request::builder()
                    .method(method)
                    .uri(path)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await;
        assert_eq!(status, StatusCode::NOT_FOUND, "path={path}");
    }

    let (status, _) = app.get("/api/v1/projects/recycle-bin").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
