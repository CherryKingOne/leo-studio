mod common;

use axum::http::StatusCode;
use common::TestApp;
use leo_app::modules::projects::service as project_service;
use serde_json::Value;

const SAMPLE: &str = "第1章 相遇\n林冲冷笑道：“武松，别来无恙。”\n武松答道：“托你的福。”";

async fn create_project(app: &TestApp, name: &str) -> String {
    let (status, body) = app
        .post_multipart(
            "/api/v3/projects",
            &[("name", name)],
            Some(("novel.txt", "text/plain", SAMPLE.as_bytes())),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    body["data"]["project_id"].as_str().unwrap().to_string()
}

async fn create_character(app: &TestApp, project_id: &str, name: &str) -> String {
    let (status, body) = send_json(
        app,
        "POST",
        &format!("/api/v3/projects/{project_id}/characters"),
        serde_json::json!({"name": name}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    body["data"]["character_id"].as_str().unwrap().to_string()
}

async fn first_chapter(app: &TestApp, project_id: &str) -> String {
    let (status, body) = app
        .get(&format!("/api/v3/projects/{project_id}/chapters"))
        .await;
    assert_eq!(status, StatusCode::OK);
    body["data"]["items"][0]["chapter_id"]
        .as_str()
        .unwrap()
        .to_string()
}

async fn send_json(app: &TestApp, method: &str, path: &str, payload: Value) -> (StatusCode, Value) {
    app.send(
        axum::http::Request::builder()
            .method(method)
            .uri(path)
            .header("content-type", "application/json")
            .body(axum::body::Body::from(payload.to_string()))
            .unwrap(),
    )
    .await
}

#[tokio::test]
async fn v3_dialogue_list_get_bind_and_unbind() {
    let app = TestApp::new(1).await;
    let project_id = create_project(&app, "台词项目").await;
    let chapter_id = first_chapter(&app, &project_id).await;
    let character_id = create_character(&app, &project_id, "林冲").await;
    let collection = format!("/api/v3/projects/{project_id}/chapters/{chapter_id}/dialogues");

    let (status, body) = app.get(&collection).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["total"], 4);
    let dialogue_id = body["data"]["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["segment_type"] == "dialogue")
        .unwrap()["dialogue_id"]
        .as_str()
        .unwrap()
        .to_string();

    let (status, body) = app.get(&format!("{collection}/{dialogue_id}")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["dialogue_id"], dialogue_id);

    let (status, body) = send_json(
        &app,
        "PUT",
        &format!("{collection}/{dialogue_id}/speaker"),
        serde_json::json!({"speaker_id": character_id}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["speaker_id"], character_id);
    assert_eq!(body["data"]["speaker_name"], "林冲");
    assert_eq!(body["data"]["binding_source"], "manual");

    let (status, body) = app.get(&format!("{collection}?binding_status=bound")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["total"], 1);

    let (status, body) = app
        .get(&format!(
            "{collection}?binding_status=unbound&segment_type=dialogue"
        ))
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["total"], 1);

    let (status, body) = app
        .delete(&format!("{collection}/{dialogue_id}/speaker"))
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["unbound"], true);

    let (status, _) = app
        .delete(&format!("{collection}/{dialogue_id}/speaker"))
        .await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn v3_dialogue_rejects_cross_project_character() {
    let app = TestApp::new(1).await;
    let project_a = create_project(&app, "项目 A").await;
    let project_b = create_project(&app, "项目 B").await;
    let chapter_id = first_chapter(&app, &project_a).await;
    let other_character = create_character(&app, &project_b, "其他项目角色").await;
    let collection = format!("/api/v3/projects/{project_a}/chapters/{chapter_id}/dialogues");
    let (_, body) = app.get(&collection).await;
    let dialogue_id = body["data"]["items"][0]["dialogue_id"]
        .as_str()
        .unwrap()
        .to_string();

    let (status, body) = send_json(
        &app,
        "PUT",
        &format!("{collection}/{dialogue_id}/speaker"),
        serde_json::json!({"speaker_id": other_character}),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["code"], 40400);
}

#[tokio::test]
async fn deleting_character_unbinds_dialogues() {
    let app = TestApp::new(1).await;
    let project_id = create_project(&app, "角色删除联动").await;
    let chapter_id = first_chapter(&app, &project_id).await;
    let character_id = create_character(&app, &project_id, "林冲").await;
    let collection = format!("/api/v3/projects/{project_id}/chapters/{chapter_id}/dialogues");
    let (_, body) = app.get(&collection).await;
    let dialogue_id = body["data"]["items"][0]["dialogue_id"]
        .as_str()
        .unwrap()
        .to_string();
    send_json(
        &app,
        "PUT",
        &format!("{collection}/{dialogue_id}/speaker"),
        serde_json::json!({"speaker_id": character_id}),
    )
    .await;

    let (status, _) = app
        .delete(&format!(
            "/api/v3/projects/{project_id}/characters/{character_id}"
        ))
        .await;
    assert_eq!(status, StatusCode::OK);

    let (status, body) = app.get(&format!("{collection}/{dialogue_id}")).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["data"]["speaker_id"].is_null());
    assert_eq!(body["data"]["binding_source"], "system");
}

#[tokio::test]
async fn rebuilding_chapter_clears_dialogue_bindings() {
    let dir = tempfile::tempdir().unwrap();
    let settings = leo_app::config::Settings {
        data_dir: dir.path().to_path_buf(),
        upload_dir: dir.path().join("uploads"),
        csv_dir: dir.path().join(".leo-studio"),
        ..leo_app::config::Settings::default()
    };
    let mut store = leo_app::storage::store::Store::new(&settings).unwrap();
    let project = project_service::create_project(
        &mut store,
        &settings,
        "章节重建",
        "novel.txt",
        SAMPLE.as_bytes(),
        None,
    )
    .unwrap();
    let chapter_id = store.contents.find_all()[0]
        .get("chapter_id")
        .unwrap()
        .clone();
    let dialogue_id = store.dialogues.find_all()[0]
        .get("dialogue_id")
        .unwrap()
        .clone();
    let character = leo_app::modules::projects::characters::service::create_character(
        &mut store,
        &project.project_id,
        &leo_app::modules::projects::characters::schema::CharacterCreateRequest {
            name: "林冲".into(),
            aliases: vec![],
            gender: None,
            age: None,
            role_type: None,
            status: None,
            description: None,
            tags: vec![],
        },
    )
    .unwrap();
    leo_app::modules::projects::dialogues::service::bind_speaker(
        &mut store,
        &project.project_id,
        &chapter_id,
        &dialogue_id,
        &leo_app::modules::projects::dialogues::schema::DialogueSpeakerBindRequest {
            speaker_id: character.character_id,
            binding_source: None,
            binding_confidence: None,
        },
    )
    .unwrap();

    project_service::update_chapter(
        &mut store,
        &project.project_id,
        &chapter_id,
        None,
        Some("第1章 新正文\n全新的内容。"),
    )
    .unwrap();
    assert!(
        store
            .dialogues
            .find_all()
            .iter()
            .all(|record| record.get("speaker_id").unwrap().is_empty())
    );
}
