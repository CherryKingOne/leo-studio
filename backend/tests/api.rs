mod common;

use axum::body::Body;
use axum::http::Request;
use common::TestApp;
use http_body_util::BodyExt;
use serde_json::json;
use tower::ServiceExt;

const SAMPLE_NOVEL: &str = "第1章 绯红之月\n痛！好痛！头好痛！狂暴的剧痛如同一柄烧红的钢铁长矛，狠狠刺入了脑海深处。\n\n第2章 宿命之枪\n左轮手枪与桌上的自杀日记，黄铜握柄还残留着体温。";

#[tokio::test]
async fn health_and_root() {
    let app = TestApp::new(1).await;
    let (status, body) = app.get("/health").await;
    assert_eq!(status, 200);
    assert_eq!(body["status"], "ok");

    let (status, body) = app.get("/api/v1/system/health").await;
    assert_eq!(status, 200);
    assert_eq!(body["status"], "ok");

    let (status, body) = app.get("/").await;
    assert_eq!(status, 200);
    assert_eq!(body["api_prefix"], "/api/v1");
    assert!(
        body["modules"]
            .as_array()
            .unwrap()
            .contains(&json!("projects"))
    );
}

#[tokio::test]
async fn request_id_in_header_and_body() {
    let app = TestApp::new(1).await;
    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/text-files")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let header = response
        .headers()
        .get("X-Request-ID")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["request_id"], header);
}

#[tokio::test]
async fn upload_text_file() {
    let app = TestApp::new(1).await;
    let (status, body) = app
        .post_multipart(
            "/api/v1/text-files/upload",
            &[("description", "笔记")],
            Some(("hello.txt", "text/plain", b"hello world")),
        )
        .await;

    assert_eq!(status, 200, "{body}");
    assert_eq!(body["code"], 0);
    assert_eq!(body["message"], "文件上传成功");
    let data = &body["data"];
    assert_eq!(data["filename"], "hello.txt");
    assert_eq!(data["size"], 11);
    assert_eq!(data["extension"], ".txt");
    assert_eq!(data["category"], "plain_text");
    assert_eq!(data["description"], "笔记");
    assert!(
        data["download_url"]
            .as_str()
            .unwrap()
            .contains(&data["file_id"].as_str().unwrap().to_string())
    );

    let (status, _) = app.get("/api/v1/text-files").await;
    assert_eq!(status, 200);
}

#[tokio::test]
async fn upload_rejects_unsupported_and_empty() {
    let app = TestApp::new(1).await;
    let (status, body) = app
        .post_multipart(
            "/api/v1/text-files/upload",
            &[],
            Some(("evil.exe", "application/octet-stream", b"MZ")),
        )
        .await;
    assert_eq!(status, 415);
    assert_eq!(body["code"], 41500);

    let (status, body) = app
        .post_multipart(
            "/api/v1/text-files/upload",
            &[],
            Some(("empty.txt", "text/plain", b"")),
        )
        .await;
    assert_eq!(status, 422);
    assert_eq!(body["code"], 40000);
}

#[tokio::test]
async fn upload_rejects_oversize() {
    let app = TestApp::new(1).await;
    let payload = vec![b'x'; 1024 * 1024 + 1024];
    let (status, body) = app
        .post_multipart(
            "/api/v1/text-files/upload",
            &[],
            Some(("big.txt", "text/plain", &payload)),
        )
        .await;
    assert_eq!(status, 413);
    assert_eq!(body["code"], 41300);
}

#[tokio::test]
async fn download_returns_content() {
    let app = TestApp::new(1).await;
    let (_, body) = app
        .post_multipart(
            "/api/v1/text-files/upload",
            &[],
            Some(("doc.txt", "text/plain", b"content-bytes")),
        )
        .await;
    let file_id = body["data"]["file_id"].as_str().unwrap();
    let (status, text) = app
        .send_raw(
            Request::builder()
                .uri(format!("/api/v1/text-files/{file_id}/download"))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(status, 200);
    assert_eq!(text, "content-bytes");
}

#[tokio::test]
async fn project_lifecycle() {
    let app = TestApp::new(1).await;
    let (status, body) = app
        .post_multipart(
            "/api/v1/projects",
            &[("name", "测试项目")],
            Some(("novel.txt", "text/plain", SAMPLE_NOVEL.as_bytes())),
        )
        .await;
    assert_eq!(status, 200);
    assert_eq!(body["message"], "项目创建成功");
    let project_id = body["data"]["project_id"].as_str().unwrap().to_string();
    assert_eq!(body["data"]["chapter_count"], 2);
    assert_eq!(body["data"]["status"], "制作中");

    let (status, body) = app.get(&format!("/api/v1/projects/{project_id}")).await;
    assert_eq!(status, 200);
    assert_eq!(body["data"]["name"], "测试项目");

    let (status, body) = app
        .get(&format!("/api/v1/projects/{project_id}/chapters"))
        .await;
    assert_eq!(status, 200);
    let chapters = body["data"]["items"].as_array().unwrap();
    assert_eq!(chapters.len(), 2);
    assert_eq!(chapters[0]["chapter_title"], "第1章 绯红之月");
    let chapter_id = chapters[0]["chapter_id"].as_str().unwrap().to_string();

    let (status, body) = app
        .get(&format!(
            "/api/v1/projects/{project_id}/chapters/{chapter_id}"
        ))
        .await;
    assert_eq!(status, 200);
    assert!(
        body["data"]["content"]
            .as_str()
            .unwrap()
            .contains("狂暴的剧痛")
    );
    assert!(!body["data"]["content"].as_str().unwrap().contains("￣▽￣"));
    assert!(!body["data"]["sentences"].as_array().unwrap().is_empty());

    let (status, body) = app
        .patch_json(
            &format!("/api/v1/projects/{project_id}"),
            r#"{"name":"新名称"}"#,
        )
        .await;
    assert_eq!(status, 200);
    assert_eq!(body["data"]["name"], "新名称");

    let (status, body) = app.delete(&format!("/api/v1/projects/{project_id}")).await;
    assert_eq!(status, 200);
    assert_eq!(body["data"]["deleted"], true);

    let (status, body) = app.get(&format!("/api/v1/projects/{project_id}")).await;
    assert_eq!(status, 404);
    assert_eq!(body["code"], 40400);

    let (status, body) = app.get("/api/v1/projects/recycle-bin").await;
    assert_eq!(status, 200);
    assert_eq!(body["data"]["items"].as_array().unwrap().len(), 1);

    let (status, _) = app
        .send(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/projects/{project_id}/restore"))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(status, 200);

    let (status, body) = app
        .delete(&format!("/api/v1/projects/{project_id}/hard"))
        .await;
    assert_eq!(status, 422);
    assert_eq!(body["code"], 40000);

    app.delete(&format!("/api/v1/projects/{project_id}")).await;

    let (status, body) = app
        .delete(&format!("/api/v1/projects/{project_id}/hard"))
        .await;
    assert_eq!(status, 200);
    assert_eq!(body["data"]["hard_deleted"], true);
}

#[tokio::test]
async fn polyphone_save_get_and_validation() {
    let app = TestApp::new(1).await;
    let (_, body) = app
        .post_multipart(
            "/api/v1/projects",
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
        .get(&format!("/api/v1/projects/{project_id}/chapters"))
        .await;
    let chapter_id = body["data"]["items"][0]["chapter_id"]
        .as_str()
        .unwrap()
        .to_string();
    let (_, body) = app
        .get(&format!(
            "/api/v1/projects/{project_id}/chapters/{chapter_id}"
        ))
        .await;
    let sentence_id = body["data"]["sentences"][0]["sentence_id"]
        .as_str()
        .unwrap()
        .to_string();

    let url = format!("/api/v1/projects/{project_id}/sentences/{sentence_id}/polyphones");
    let (status, body) = app
        .put_json(
            &url,
            r#"{"annotations":[{"char_index":1,"character":"行","pinyin":"xíng"}]}"#,
        )
        .await;
    assert_eq!(status, 200);
    assert_eq!(body["message"], "多音字标注保存成功");
    let item = &body["data"]["items"][0];
    assert_eq!(item["pinyin_base"], "xing");
    assert_eq!(item["tone"], 2);
    assert_eq!(item["tts_pinyin"], "xing2");

    let (_, body) = app.get(&url).await;
    assert_eq!(body["data"]["total"], 1);

    let (status, body) = app
        .put_json(
            &url,
            r#"{"annotations":[{"char_index":1,"character":"行","pinyin":"háng"}]}"#,
        )
        .await;
    assert_eq!(status, 200);
    assert_eq!(body["data"]["items"][0]["pinyin"], "háng");

    let (status, body) = app
        .put_json(
            &url,
            r#"{"annotations":[{"char_index":999,"character":"行","pinyin":"xíng"}]}"#,
        )
        .await;
    assert_eq!(status, 422);
    assert!(
        body["message"]
            .as_str()
            .unwrap()
            .contains("超出句子长度范围")
    );

    let (status, _) = app.delete(&url).await;
    assert_eq!(status, 200);
}

#[tokio::test]
async fn append_chapters_dry_run_and_overwrite() {
    let app = TestApp::new(2).await;
    let (_, body) = app
        .post_multipart(
            "/api/v1/projects",
            &[("name", "连载项目")],
            Some(("novel.txt", "text/plain", SAMPLE_NOVEL.as_bytes())),
        )
        .await;
    let project_id = body["data"]["project_id"].as_str().unwrap().to_string();

    let increment = "第1章 绯红之月\n极短正文。\n\n第3章 新世界\n崭新的一章。";
    let (status, body) = app
        .post_multipart(
            &format!("/api/v1/projects/{project_id}/chapters/upload"),
            &[("dry_run", "true")],
            Some(("inc.txt", "text/plain", increment.as_bytes())),
        )
        .await;
    assert_eq!(status, 200);
    assert_eq!(body["data"]["has_conflicts"], true);
    assert_eq!(body["data"]["conflicts"][0]["incoming_word_count"], 5);
    assert!(body["data"]["conflicts"][0]["word_diff"].as_i64().unwrap() < 0);
    assert_eq!(body["data"]["total_chapters"], 3);

    let (_, body) = app
        .get(&format!("/api/v1/projects/{project_id}/chapters"))
        .await;
    assert_eq!(body["data"]["total"], 2);

    let (status, body) = app
        .post_multipart(
            &format!("/api/v1/projects/{project_id}/chapters/upload"),
            &[("overwrite_existing", "true")],
            Some(("inc.txt", "text/plain", increment.as_bytes())),
        )
        .await;
    assert_eq!(status, 200);
    assert_eq!(body["data"]["skipped_chapters"], 1);
    assert_eq!(body["data"]["added_chapters"], 1);
}

#[tokio::test]
async fn clear_recycle_bin_cascades_files() {
    let app = TestApp::new(1).await;
    let (_, body) = app
        .post_multipart(
            "/api/v1/projects",
            &[("name", "回收项目")],
            Some(("novel.txt", "text/plain", SAMPLE_NOVEL.as_bytes())),
        )
        .await;
    let project_id = body["data"]["project_id"].as_str().unwrap().to_string();
    let file_id = body["data"]["file_id"].as_str().unwrap().to_string();

    app.delete(&format!("/api/v1/projects/{project_id}")).await;
    let (status, body) = app.delete("/api/v1/projects/recycle-bin").await;
    assert_eq!(status, 200);
    assert_eq!(body["data"]["cleared"], true);
    assert!(body["data"]["cleared_count"].as_u64().unwrap() >= 1);

    let (status, body) = app.get(&format!("/api/v1/text-files/{file_id}")).await;
    assert_eq!(status, 404);
    assert_eq!(body["code"], 40400);
}
