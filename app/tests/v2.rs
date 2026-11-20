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
            r#"{"status":"已完成"}"#,
        )
        .await;
    assert_eq!(status, 200);
    assert_eq!(body["data"]["status"], "已完成");

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
    assert_eq!(status, 422);
    assert_eq!(body["code"], 40000);

    app.delete(&format!("/api/v2/projects/{project_id}")).await;

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
async fn v2_append_chapters_dry_run_then_write() {
    let app = TestApp::new(1).await;

    let (_, body) = app
        .post_multipart(
            "/api/v2/projects",
            &[("name", "追更项目")],
            Some(("novel.txt", "text/plain", SAMPLE_NOVEL.as_bytes())),
        )
        .await;
    let project_id = body["data"]["project_id"].as_str().unwrap().to_string();

    let (_, body) = app
        .get(&format!("/api/v2/projects/{project_id}/chapters"))
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
            &format!("/api/v2/projects/{project_id}/chapters"),
            &[("dry_run", "true")],
            Some(("inc.txt", "text/plain", increment.as_bytes())),
        )
        .await;
    assert_eq!(status, 200);
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
        .get(&format!("/api/v2/projects/{project_id}/chapters"))
        .await;
    assert_eq!(body["data"]["total"], 2);

    let (status, body) = app
        .post_multipart(
            &format!("/api/v2/projects/{project_id}/chapters"),
            &[("dry_run", "false")],
            Some(("inc.txt", "text/plain", increment.as_bytes())),
        )
        .await;
    assert_eq!(status, 200);
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

    let (_, body) = app.get(&format!("/api/v2/projects/{project_id}")).await;
    assert_eq!(body["data"]["total_words"], projected_total_words);

    let (_, body) = app
        .get(&format!("/api/v2/projects/{project_id}/chapters"))
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
async fn v2_append_inserts_chapter_by_number() {
    let app = TestApp::new(1).await;
    let novel = "第2章 第二章\n第二节内容。\n\n第3章 第三章\n第三节内容。";

    let (_, body) = app
        .post_multipart(
            "/api/v2/projects",
            &[("name", "顺序项目")],
            Some(("novel.txt", "text/plain", novel.as_bytes())),
        )
        .await;
    let project_id = body["data"]["project_id"].as_str().unwrap().to_string();

    let (status, body) = app
        .post_multipart(
            &format!("/api/v2/projects/{project_id}/chapters"),
            &[],
            Some((
                "chapter1.txt",
                "text/plain",
                "第1章 第一章\n第一节内容。".as_bytes(),
            )),
        )
        .await;
    assert_eq!(status, 200);
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
        .get(&format!("/api/v2/projects/{project_id}/chapters"))
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
async fn v2_chapter_polyphones_list() {
    let app = TestApp::new(1).await;

    let (_, body) = app
        .post_multipart(
            "/api/v2/projects",
            &[("name", "多音字章节项目")],
            Some(("novel.txt", "text/plain", SAMPLE_NOVEL.as_bytes())),
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

    let (status, _) = app
        .put_json(
            &format!("/api/v2/projects/{project_id}/sentences/{sentence_id}/polyphones"),
            r#"{"annotations":[{"char_index":2,"character":"好","pinyin":"hǎo"}]}"#,
        )
        .await;
    assert_eq!(status, 200);

    let (status, body) = app
        .get(&format!(
            "/api/v2/projects/{project_id}/chapters/{chapter_id}/polyphones"
        ))
        .await;
    assert_eq!(status, 200);
    assert_eq!(body["data"]["chapter_id"], chapter_id);
    assert_eq!(body["data"]["sentence_count"], 1);
    assert_eq!(body["data"]["total"], 1);
    assert_eq!(body["data"]["items"][0]["sentence_id"], sentence_id);
    assert_eq!(body["data"]["items"][0]["polyphones"][0]["character"], "好");
}

#[tokio::test]
async fn v2_delete_single_polyphone() {
    let app = TestApp::new(1).await;

    let (_, body) = app
        .post_multipart(
            "/api/v2/projects",
            &[("name", "单条删除项目")],
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

    let url = format!(
        "/api/v2/projects/{project_id}/chapters/{chapter_id}/sentences/{sentence_id}/polyphones"
    );
    let (status, _) = app
        .put_json(
            &url,
            r#"{"annotations":[
                {"char_index":1,"character":"行","pinyin":"xíng"},
                {"char_index":6,"character":"行","pinyin":"xíng"}
            ]}"#,
        )
        .await;
    assert_eq!(status, 200);

    let (status, body) = app.delete(&format!("{url}/1")).await;
    assert_eq!(status, 200);
    assert_eq!(body["data"]["char_index"], 1);
    assert_eq!(body["data"]["deleted"], true);

    let (_, body) = app.get(&url).await;
    assert_eq!(body["data"]["total"], 1);
    assert_eq!(body["data"]["items"][0]["char_index"], 6);

    let (status, _) = app.delete(&format!("{url}/1")).await;
    assert_eq!(status, 404);
}

#[tokio::test]
async fn v2_delete_chapter_cascades_and_updates_stats() {
    let app = TestApp::new(1).await;

    let (_, body) = app
        .post_multipart(
            "/api/v2/projects",
            &[("name", "章节删除项目")],
            Some(("novel.txt", "text/plain", SAMPLE_NOVEL.as_bytes())),
        )
        .await;
    let project_id = body["data"]["project_id"].as_str().unwrap().to_string();

    let (_, body) = app.get(&format!("/api/v2/projects/{project_id}")).await;
    let total_words_before = body["data"]["total_words"].as_i64().unwrap();

    let (_, body) = app
        .get(&format!("/api/v2/projects/{project_id}/chapters"))
        .await;
    assert_eq!(body["data"]["total"], 2);
    let first = body["data"]["items"][0].clone();
    let chapter_id = first["chapter_id"].as_str().unwrap().to_string();

    let (status, body) = app
        .delete(&format!(
            "/api/v2/projects/{project_id}/chapters/{chapter_id}"
        ))
        .await;
    assert_eq!(status, 200);
    assert_eq!(body["data"]["deleted"], true);
    assert_eq!(body["data"]["chapter_id"], chapter_id);
    assert_eq!(body["data"]["chapter_count"], 1);
    let deleted_total_words = body["data"]["total_words"].as_i64().unwrap();
    assert!(deleted_total_words < total_words_before);

    let (_, body) = app
        .get(&format!("/api/v2/projects/{project_id}/chapters"))
        .await;
    assert_eq!(body["data"]["total"], 1);
    assert_ne!(body["data"]["items"][0]["chapter_id"], chapter_id);
    assert_eq!(body["data"]["items"][0]["word_count"], deleted_total_words);

    let (status, body) = app
        .get(&format!(
            "/api/v2/projects/{project_id}/chapters/{chapter_id}"
        ))
        .await;
    assert_eq!(status, 404);
    assert_eq!(body["code"], 40400);

    let (status, _) = app
        .delete(&format!(
            "/api/v2/projects/{project_id}/chapters/{chapter_id}"
        ))
        .await;
    assert_eq!(status, 404);
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

    let url = format!(
        "/api/v2/projects/{project_id}/chapters/{chapter_id}/sentences/{sentence_id}/polyphones"
    );
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

    let (status, _) = app
        .put_json(
            &format!(
                "/api/v2/projects/{project_id}/chapters/missing-chapter/sentences/{sentence_id}/polyphones"
            ),
            r#"{"annotations":[{"char_index":1,"character":"行","pinyin":"xíng"}]}"#,
        )
        .await;
    assert_eq!(status, 404);

    let (status, _) = app.delete(&url).await;
    assert_eq!(status, 200);
}

#[tokio::test]
async fn v2_import_survives_project_create() {
    let dir = tempfile::tempdir().unwrap();
    let app = TestApp::with_dir(dir.path(), 1).await;

    let (status, body) = app
        .post_multipart(
            "/api/v2/import",
            &[],
            Some(("imported.txt", "text/plain", SAMPLE_NOVEL.as_bytes())),
        )
        .await;
    assert_eq!(status, 200);
    let imported_id = body["data"]["id"].as_str().unwrap().to_string();

    let (_, body) = app
        .post_multipart(
            "/api/v2/projects",
            &[("name", "后建项目")],
            Some(("novel.txt", "text/plain", SAMPLE_NOVEL.as_bytes())),
        )
        .await;
    assert_eq!(body["message"], "项目创建成功");
    let project_id = body["data"]["project_id"].as_str().unwrap().to_string();

    // 再导入一次，确认插件落盘不会抹掉 REST 新建的项目
    let (status, _) = app
        .post_multipart(
            "/api/v2/import",
            &[],
            Some(("second.txt", "text/plain", SAMPLE_NOVEL.as_bytes())),
        )
        .await;
    assert_eq!(status, 200);
    drop(app);

    let app = TestApp::with_dir(dir.path(), 1).await;
    let (status, body) = app.get(&format!("/api/v2/export/{imported_id}")).await;
    assert_eq!(status, 200, "新建项目后导入的剧本不应丢失: {body}");

    let (status, body) = app.get(&format!("/api/v2/projects/{project_id}/chapters")).await;
    assert_eq!(status, 200);
    assert_eq!(body["data"]["total"], 2);
}
