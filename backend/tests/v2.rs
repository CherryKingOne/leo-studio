mod common;

use axum::body::Body;
use axum::http::Request;
use common::TestApp;

const SAMPLE_NOVEL: &str =
    "第1章 绯红之月\n痛！好痛！头好痛！\n\n第2章 宿命之枪\n左轮手枪与桌上的日记。";

#[tokio::test]
async fn v2_health_and_index() {
    let app = TestApp::new(1).await;

    let (status, body) = app.get("/api/v2/health").await;
    assert_eq!(status, 200);
    assert_eq!(body["status"], "ok");

    let (status, body) = app.get("/api/v2").await;
    assert_eq!(status, 200);
    assert_eq!(body["api_prefix"], "/api/v2");
    assert!(
        body["modules"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("projects"))
    );
}

#[tokio::test]
async fn v2_project_crud_and_filters() {
    let app = TestApp::new(1).await;

    let (_, body) = app
        .post_multipart(
            "/api/v2/projects",
            &[("name", "V2 项目")],
            Some(("novel.txt", "text/plain", SAMPLE_NOVEL.as_bytes())),
        )
        .await;
    assert_eq!(body["message"], "项目创建成功");
    let project_id = body["data"]["project_id"].as_str().unwrap().to_string();

    let (status, body) = app.get(&format!("/api/v2/projects/{project_id}")).await;
    assert_eq!(status, 200);
    assert_eq!(body["data"]["name"], "V2 项目");

    let (status, body) = app
        .get("/api/v2/projects?keyword=V2&status=%E5%88%B6%E4%BD%9C%E4%B8%AD")
        .await;
    assert_eq!(status, 200);
    assert_eq!(body["data"]["total"], 1);

    let (status, body) = app
        .patch_json(
            &format!("/api/v2/projects/{project_id}"),
            r#"{"status":"母带就绪"}"#,
        )
        .await;
    assert_eq!(status, 200);
    assert_eq!(body["data"]["status"], "母带就绪");

    let (status, body) = app
        .get(&format!("/api/v2/projects/{project_id}/chapters"))
        .await;
    assert_eq!(status, 200);
    assert_eq!(body["data"]["total"], 2);

    let (status, _) = app.delete(&format!("/api/v2/projects/{project_id}")).await;
    assert_eq!(status, 200);
}

#[tokio::test]
async fn v2_recycle_bin_restore_and_hard_delete() {
    let app = TestApp::new(1).await;

    let (_, body) = app
        .post_multipart(
            "/api/v2/projects",
            &[("name", "回收项目")],
            Some(("novel.txt", "text/plain", SAMPLE_NOVEL.as_bytes())),
        )
        .await;
    let project_id = body["data"]["project_id"].as_str().unwrap().to_string();

    app.delete(&format!("/api/v2/projects/{project_id}")).await;

    let (status, body) = app.get("/api/v2/projects/recycle-bin").await;
    assert_eq!(status, 200);
    assert_eq!(body["data"]["total"], 1);

    let (status, _) = app
        .send(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v2/projects/{project_id}/restore"))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(status, 200);

    let (status, body) = app
        .delete(&format!("/api/v2/projects/{project_id}/hard"))
        .await;
    assert_eq!(status, 200);
    assert_eq!(body["data"]["hard_deleted"], true);

    let (status, body) = app.get(&format!("/api/v2/projects/{project_id}")).await;
    assert_eq!(status, 404);
    assert_eq!(body["code"], 40400);
}

#[tokio::test]
async fn v2_polyphone_save_and_clear() {
    let app = TestApp::new(1).await;

    let (_, body) = app
        .post_multipart(
            "/api/v2/projects",
            &[("name", "多音字项目")],
            Some((
                "novel.txt",
                "text/plain",
                "第1章 城市一角\n人行道上有一行桌子。".as_bytes(),
            )),
        )
        .await;
    let project_id = body["data"]["project_id"].as_str().unwrap().to_string();

    let (_, body) = app
        .get(&format!("/api/v2/projects/{project_id}/chapters"))
        .await;
    let chapter_id = body["data"]["items"][0]["chapter_id"]
        .as_str()
        .unwrap()
        .to_string();
    let (_, body) = app
        .get(&format!(
            "/api/v2/projects/{project_id}/chapters/{chapter_id}"
        ))
        .await;
    let sentence_id = body["data"]["sentences"][0]["sentence_id"]
        .as_str()
        .unwrap()
        .to_string();

    let url = format!("/api/v2/projects/{project_id}/sentences/{sentence_id}/polyphones");
    let (status, body) = app
        .put_json(
            &url,
            r#"{"annotations":[{"char_index":1,"character":"行","pinyin":"xíng"}]}"#,
        )
        .await;
    assert_eq!(status, 200);
    assert_eq!(body["data"]["items"][0]["tts_pinyin"], "xing2");

    let (_, body) = app.get(&url).await;
    assert_eq!(body["data"]["total"], 1);

    let (status, _) = app.delete(&url).await;
    assert_eq!(status, 200);
}
