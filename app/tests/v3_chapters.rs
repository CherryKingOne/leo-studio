mod common;

use axum::http::StatusCode;
use common::TestApp;

const SAMPLE_NOVEL: &str =
    "第1章 绯红之月\n痛！好痛！头好痛！\n\n第2章 宿命之枪\n左轮手枪与桌上的日记。";

async fn create_project(app: &TestApp, name: &str, novel: &str) -> String {
    let (status, body) = app
        .post_multipart(
            "/api/v3/projects",
            &[("name", name)],
            Some(("novel.txt", "text/plain", novel.as_bytes())),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    body["data"]["project_id"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn v3_chapter_list_get_and_filter() {
    let app = TestApp::new(1).await;
    let project_id = create_project(&app, "章节查询项目", SAMPLE_NOVEL).await;

    let (status, body) = app
        .get(&format!("/api/v3/projects/{project_id}/chapters"))
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["total"], 2);
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
    assert_eq!(body["data"]["chapter_id"], chapter_id);
    assert!(!body["data"]["sentences"].as_array().unwrap().is_empty());

    let (status, body) = app
        .get(&format!(
            "/api/v3/projects/{project_id}/chapters?keyword=绯红"
        ))
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["total"], 1);
}

#[tokio::test]
async fn v3_append_chapters_dry_run_then_write() {
    let app = TestApp::new(1).await;
    let project_id = create_project(&app, "追更项目", SAMPLE_NOVEL).await;

    let (_, body) = app
        .get(&format!("/api/v3/projects/{project_id}/chapters"))
        .await;
    let existing_total_words: i64 = body["data"]["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["word_count"].as_i64().unwrap())
        .sum();

    let increment = "第3章 新世界\n崭新的一章。";
    let (status, body) = app
        .post_multipart(
            &format!("/api/v3/projects/{project_id}/chapters"),
            &[("dry_run", "true")],
            Some(("inc.txt", "text/plain", increment.as_bytes())),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["dry_run"], true);
    assert_eq!(body["data"]["added_chapters"], 1);
    assert_eq!(body["data"]["total_chapters"], 3);
    let projected_total_words = body["data"]["total_words"].as_i64().unwrap();
    assert!(projected_total_words > existing_total_words);
    let dry_added = body["data"]["chapters"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["action"] == "added")
        .unwrap();
    assert!(dry_added["chapter_id"].is_null());
    assert!(dry_added["chapter_index"].is_null());

    let (_, body) = app
        .get(&format!("/api/v3/projects/{project_id}/chapters"))
        .await;
    assert_eq!(body["data"]["total"], 2);

    let (status, body) = app
        .post_multipart(
            &format!("/api/v3/projects/{project_id}/chapters"),
            &[("dry_run", "false")],
            Some(("inc.txt", "text/plain", increment.as_bytes())),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["dry_run"], false);
    assert_eq!(body["data"]["added_chapters"], 1);
    assert_eq!(body["data"]["total_words"], projected_total_words);
    let added_chapter_id = body["data"]["chapters"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["action"] == "added")
        .unwrap()["chapter_id"]
        .as_str()
        .unwrap()
        .to_string();

    let (_, body) = app.get(&format!("/api/v3/projects/{project_id}")).await;
    assert_eq!(body["data"]["total_words"], projected_total_words);

    let (_, body) = app
        .get(&format!("/api/v3/projects/{project_id}/chapters"))
        .await;
    assert_eq!(body["data"]["total"], 3);
    assert!(
        body["data"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["chapter_id"] == added_chapter_id)
    );
}

#[tokio::test]
async fn v3_append_inserts_chapter_by_number() {
    let app = TestApp::new(1).await;
    let novel = "第2章 第二章\n第二节内容。\n\n第3章 第三章\n第三节内容。";
    let project_id = create_project(&app, "顺序项目", novel).await;

    let (status, body) = app
        .post_multipart(
            &format!("/api/v3/projects/{project_id}/chapters"),
            &[],
            Some((
                "chapter1.txt",
                "text/plain",
                "第1章 第一章\n第一节内容。".as_bytes(),
            )),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["added_chapters"], 1);
    assert_eq!(body["data"]["total_chapters"], 3);
    let added_chapter_id = body["data"]["chapters"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["action"] == "added")
        .unwrap()["chapter_id"]
        .as_str()
        .unwrap()
        .to_string();

    let (_, body) = app
        .get(&format!("/api/v3/projects/{project_id}/chapters"))
        .await;
    let items = body["data"]["items"].as_array().unwrap();
    assert_eq!(items.len(), 3);
    assert_eq!(items[0]["chapter_id"], added_chapter_id);
    assert_eq!(items[0]["chapter_index"], 0);
    assert_eq!(items[0]["chapter_title"], "第1章 第一章");
    assert_eq!(items[1]["chapter_index"], 1);
    assert_eq!(items[1]["chapter_title"], "第2章 第二章");
    assert_eq!(items[2]["chapter_index"], 2);
    assert_eq!(items[2]["chapter_title"], "第3章 第三章");
}

#[tokio::test]
async fn v3_delete_chapter_cascades_and_updates_stats() {
    let app = TestApp::new(1).await;
    let project_id = create_project(&app, "章节删除项目", SAMPLE_NOVEL).await;

    let (_, body) = app.get(&format!("/api/v3/projects/{project_id}")).await;
    let total_words_before = body["data"]["total_words"].as_i64().unwrap();

    let (_, body) = app
        .get(&format!("/api/v3/projects/{project_id}/chapters"))
        .await;
    assert_eq!(body["data"]["total"], 2);
    let chapter_id = body["data"]["items"][0]["chapter_id"]
        .as_str()
        .unwrap()
        .to_string();

    let (status, body) = app
        .delete(&format!(
            "/api/v3/projects/{project_id}/chapters/{chapter_id}"
        ))
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["deleted"], true);
    assert_eq!(body["data"]["chapter_id"], chapter_id);
    assert_eq!(body["data"]["chapter_count"], 1);
    let deleted_total_words = body["data"]["total_words"].as_i64().unwrap();
    assert!(deleted_total_words < total_words_before);

    let (_, body) = app
        .get(&format!("/api/v3/projects/{project_id}/chapters"))
        .await;
    assert_eq!(body["data"]["total"], 1);
    assert_ne!(body["data"]["items"][0]["chapter_id"], chapter_id);
    assert_eq!(body["data"]["items"][0]["word_count"], deleted_total_words);

    let (status, body) = app
        .get(&format!(
            "/api/v3/projects/{project_id}/chapters/{chapter_id}"
        ))
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["code"], 40400);

    let (status, _) = app
        .delete(&format!(
            "/api/v3/projects/{project_id}/chapters/{chapter_id}"
        ))
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn legacy_chapter_routes_are_removed() {
    let app = TestApp::new(1).await;

    for prefix in ["/api/v2", "/api/v1"] {
        let collection = format!("{prefix}/projects/missing/chapters");
        let (status, _) = app.get(&collection).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "path={collection}");

        let (status, _) = app
            .post_multipart(
                &collection,
                &[],
                Some(("chapter.txt", "text/plain", SAMPLE_NOVEL.as_bytes())),
            )
            .await;
        assert_eq!(status, StatusCode::NOT_FOUND, "path={collection}");

        let item = format!("{collection}/missing");
        let (status, _) = app.get(&item).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "path={item}");

        let (status, _) = app.delete(&item).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "path={item}");
    }
}
