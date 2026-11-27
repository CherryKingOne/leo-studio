mod common;

use axum::http::StatusCode;
use common::TestApp;

const SAMPLE_NOVEL: &str =
    "第1章 绯红之月\n痛！好痛！头好痛！\n\n第2章 宿命之枪\n左轮手枪与桌上的日记。";
const CITY_NOVEL: &str = "第1章 城市一角\n人行道上有一行桌子。";

async fn create_sentence(app: &TestApp, name: &str, novel: &str) -> (String, String, String) {
    let (status, body) = app
        .post_multipart(
            "/api/v3/projects",
            &[("name", name)],
            Some(("novel.txt", "text/plain", novel.as_bytes())),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let project_id = body["data"]["project_id"].as_str().unwrap().to_string();

    let (status, body) = app
        .get(&format!("/api/v3/projects/{project_id}/chapters"))
        .await;
    assert_eq!(status, StatusCode::OK);
    let chapter_id = body["data"]["items"][0]["chapter_id"]
        .as_str()
        .unwrap()
        .to_string();

    let (status, body) = app
        .get(&format!(
            "/api/v3/projects/{project_id}/chapters/{chapter_id}"
        ))
        .await;
    assert_eq!(status, StatusCode::OK);
    let sentence_id = body["data"]["sentences"][0]["sentence_id"]
        .as_str()
        .unwrap()
        .to_string();

    (project_id, chapter_id, sentence_id)
}

fn sentence_url(project_id: &str, chapter_id: &str, sentence_id: &str) -> String {
    format!(
        "/api/v3/projects/{project_id}/chapters/{chapter_id}/sentences/{sentence_id}/polyphones"
    )
}

#[tokio::test]
async fn v3_chapter_polyphones_list() {
    let app = TestApp::new(1).await;
    let (project_id, chapter_id, sentence_id) =
        create_sentence(&app, "多音字章节项目", SAMPLE_NOVEL).await;
    let url = sentence_url(&project_id, &chapter_id, &sentence_id);

    let (status, _) = app
        .put_json(
            &url,
            r#"{"annotations":[{"char_index":2,"character":"好","pinyin":"hǎo"}]}"#,
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    let (status, body) = app
        .get(&format!(
            "/api/v3/projects/{project_id}/chapters/{chapter_id}/polyphones"
        ))
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["chapter_id"], chapter_id);
    assert_eq!(body["data"]["sentence_count"], 1);
    assert_eq!(body["data"]["total"], 1);
    assert_eq!(body["data"]["items"][0]["sentence_id"], sentence_id);
    assert_eq!(body["data"]["items"][0]["polyphones"][0]["character"], "好");
}

#[tokio::test]
async fn v3_delete_single_polyphone() {
    let app = TestApp::new(1).await;
    let (project_id, chapter_id, sentence_id) =
        create_sentence(&app, "单条删除项目", CITY_NOVEL).await;
    let url = sentence_url(&project_id, &chapter_id, &sentence_id);

    let (status, _) = app
        .put_json(
            &url,
            r#"{"annotations":[
                {"char_index":1,"character":"行","pinyin":"xíng"},
                {"char_index":6,"character":"行","pinyin":"xíng"}
            ]}"#,
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    let (status, body) = app.delete(&format!("{url}/items/1")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["char_index"], 1);
    assert_eq!(body["data"]["deleted"], true);

    let (_, body) = app.get(&url).await;
    assert_eq!(body["data"]["total"], 1);
    assert_eq!(body["data"]["items"][0]["char_index"], 6);

    let (status, _) = app.delete(&format!("{url}/items/1")).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn v3_polyphone_save_and_clear() {
    let app = TestApp::new(1).await;
    let (project_id, chapter_id, sentence_id) =
        create_sentence(&app, "多音字项目", CITY_NOVEL).await;
    let url = sentence_url(&project_id, &chapter_id, &sentence_id);

    let (status, body) = app
        .put_json(
            &url,
            r#"{"annotations":[{"char_index":1,"character":"行","pinyin":"xíng"}]}"#,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["items"][0]["tts_pinyin"], "xing2");

    let (_, body) = app.get(&url).await;
    assert_eq!(body["data"]["total"], 1);

    let (status, _) = app
        .put_json(
            &format!(
                "/api/v3/projects/{project_id}/chapters/missing-chapter/sentences/{sentence_id}/polyphones"
            ),
            r#"{"annotations":[{"char_index":1,"character":"行","pinyin":"xíng"}]}"#,
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let (status, _) = app.delete(&url).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn legacy_polyphone_routes_are_removed() {
    let app = TestApp::new(1).await;
    let payload = r#"{"annotations":[{"char_index":1,"character":"行","pinyin":"xíng"}]}"#;

    for prefix in ["/api/v2", "/api/v1"] {
        let chapter_url = format!("{prefix}/projects/p/chapters/c/polyphones");
        let (status, _) = app.get(&chapter_url).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "path={chapter_url}");

        let sentence_url = format!("{prefix}/projects/p/chapters/c/sentences/s/polyphones");
        let (status, _) = app.get(&sentence_url).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "path={sentence_url}");
        let (status, _) = app.put_json(&sentence_url, payload).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "path={sentence_url}");
        let (status, _) = app.delete(&sentence_url).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "path={sentence_url}");

        let item_url = format!("{sentence_url}/1");
        let (status, _) = app.delete(&item_url).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "path={item_url}");

        let short_url = format!("{prefix}/projects/p/sentences/s/polyphones");
        let (status, _) = app.get(&short_url).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "path={short_url}");
        let (status, _) = app.put_json(&short_url, payload).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "path={short_url}");
        let (status, _) = app.delete(&short_url).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "path={short_url}");
    }
}
