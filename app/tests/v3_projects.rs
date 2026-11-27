mod common;

use axum::http::StatusCode;
use common::TestApp;

const SAMPLE_NOVEL: &str =
    "第1章 绯红之月\n痛！好痛！头好痛！\n\n第2章 宿命之枪\n左轮手枪与桌上的日记。";

#[tokio::test]
async fn v3_project_crud_and_filters() {
    let app = TestApp::new(1).await;

    let (status, body) = app
        .post_multipart(
            "/api/v3/projects",
            &[("name", "V3 项目")],
            Some(("novel.txt", "text/plain", SAMPLE_NOVEL.as_bytes())),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["message"], "项目创建成功");
    let project_id = body["data"]["project_id"].as_str().unwrap().to_string();

    let (status, body) = app.get(&format!("/api/v3/projects/{project_id}")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["name"], "V3 项目");

    let (status, body) = app
        .get("/api/v3/projects?keyword=V3&status=%E5%88%B6%E4%BD%9C%E4%B8%AD")
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["total"], 1);

    let (status, body) = app
        .patch_json(
            &format!("/api/v3/projects/{project_id}"),
            r#"{"status":"已完成"}"#,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["status"], "已完成");

    let (status, _) = app.delete(&format!("/api/v3/projects/{project_id}")).await;
    assert_eq!(status, StatusCode::OK);

    let (status, body) = app.get(&format!("/api/v3/projects/{project_id}")).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["code"], 40400);
}

#[tokio::test]
async fn v3_project_validation() {
    let app = TestApp::new(1).await;

    let (status, body) = app
        .post_multipart(
            "/api/v3/projects",
            &[("name", "")],
            Some(("novel.txt", "text/plain", SAMPLE_NOVEL.as_bytes())),
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["code"], 40000);

    let (status, body) = app.get("/api/v3/projects?page=0").await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["code"], 40000);

    let (status, body) = app.patch_json("/api/v3/projects/missing", "{}").await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["code"], 40000);
}

#[tokio::test]
async fn v2_project_routes_are_removed() {
    let app = TestApp::new(1).await;

    let (status, _) = app.get("/api/v2/projects").await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let (status, _) = app
        .post_multipart(
            "/api/v2/projects",
            &[("name", "旧接口")],
            Some(("novel.txt", "text/plain", SAMPLE_NOVEL.as_bytes())),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let (status, _) = app.get("/api/v2/projects/missing").await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let (status, _) = app
        .patch_json("/api/v2/projects/missing", r#"{"name":"旧接口"}"#)
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let (status, _) = app.delete("/api/v2/projects/missing").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
