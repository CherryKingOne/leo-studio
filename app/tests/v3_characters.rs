mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::TestApp;
use leo_app::config::Settings;
use leo_app::modules::projects::characters::schema::CharacterCreateRequest;
use leo_app::modules::projects::characters::service as character_service;
use leo_app::modules::projects::service as project_service;
use leo_app::storage::store::Store;
use serde_json::Value;

const SAMPLE_NOVEL: &str = "第1章 开端\n林冲来到梁山。\n第2章 重逢\n武松遇见林冲。";

async fn create_project(app: &TestApp, name: &str) -> String {
    let (status, body) = app
        .post_multipart(
            "/api/v3/projects",
            &[("name", name)],
            Some(("novel.txt", "text/plain", SAMPLE_NOVEL.as_bytes())),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    body["data"]["project_id"].as_str().unwrap().to_string()
}

async fn send_json(app: &TestApp, method: &str, path: &str, payload: Value) -> (StatusCode, Value) {
    app.send(
        Request::builder()
            .method(method)
            .uri(path)
            .header("content-type", "application/json")
            .body(Body::from(payload.to_string()))
            .unwrap(),
    )
    .await
}

#[tokio::test]
async fn v3_character_crud_and_status_filter() {
    let app = TestApp::new(1).await;
    let project_id = create_project(&app, "角色 CRUD 项目").await;
    let collection = format!("/api/v3/projects/{project_id}/characters");

    let (status, body) = send_json(
        &app,
        "POST",
        &collection,
        serde_json::json!({
            "name": "林冲",
            "aliases": ["豹子头", "林教头"],
            "gender": "male",
            "age": "三十出头",
            "role_type": "protagonist",
            "description": "原东京八十万禁军枪棒教头。",
            "tags": ["武将", "梁山"]
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let character_id = body["data"]["character_id"].as_str().unwrap().to_string();
    assert_eq!(body["data"]["project_id"], project_id);
    assert_eq!(body["data"]["aliases"][0], "豹子头");

    let (status, body) = app.get(&collection).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["total"], 1);

    let item_url = format!("{collection}/{character_id}");
    let (status, body) = app.get(&item_url).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["name"], "林冲");

    let (status, body) = send_json(
        &app,
        "PATCH",
        &item_url,
        serde_json::json!({
            "status": "archived",
            "age": null,
            "tags": []
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["status"], "archived");
    assert!(body["data"]["age"].is_null());
    assert_eq!(body["data"]["tags"], serde_json::json!([]));

    let (status, body) = app.get(&collection).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["total"], 0);

    let (status, body) = app.get(&format!("{collection}?status=archived")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["total"], 1);

    let (status, body) = app.delete(&item_url).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["deleted"], true);

    let (status, _) = app.get(&item_url).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn v3_character_conflicts_and_project_isolation() {
    let app = TestApp::new(1).await;
    let project_a = create_project(&app, "项目 A").await;
    let project_b = create_project(&app, "项目 B").await;
    let collection_a = format!("/api/v3/projects/{project_a}/characters");
    let collection_b = format!("/api/v3/projects/{project_b}/characters");

    let (status, body) = send_json(
        &app,
        "POST",
        &collection_a,
        serde_json::json!({"name": "林冲", "aliases": ["豹子头"]}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let character_id = body["data"]["character_id"].as_str().unwrap().to_string();

    let (status, body) = send_json(
        &app,
        "POST",
        &collection_a,
        serde_json::json!({"name": "另一个角色", "aliases": ["林冲"]}),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["code"], 40900);

    let (status, _) = send_json(
        &app,
        "POST",
        &collection_b,
        serde_json::json!({"name": "林冲"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, _) = app
        .get(&format!(
            "/api/v3/projects/{project_b}/characters/{character_id}"
        ))
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn v3_character_list_filters_and_pagination() {
    let app = TestApp::new(1).await;
    let project_id = create_project(&app, "角色过滤项目").await;
    let collection = format!("/api/v3/projects/{project_id}/characters");

    for (name, gender, role_type, tag) in [
        ("林冲", "male", "protagonist", "武将"),
        ("武松", "male", "supporting", "武将"),
        ("扈三娘", "female", "supporting", "将门"),
    ] {
        let (status, _) = send_json(
            &app,
            "POST",
            &collection,
            serde_json::json!({
                "name": name,
                "gender": gender,
                "role_type": role_type,
                "tags": [tag]
            }),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
    }

    let (status, body) = app
        .get(&format!(
            "{collection}?gender=male&role_type=supporting&tag=武将&page=1&page_size=1&sort=name&order=asc"
        ))
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["total"], 1);
    assert_eq!(body["data"]["items"][0]["name"], "武松");

    let (status, body) = app.get(&format!("{collection}?keyword=扈三")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["total"], 1);

    let (status, body) = app.get(&format!("{collection}?page=0")).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["code"], 40000);
}

#[test]
fn project_hard_delete_cascades_characters() {
    let dir = tempfile::tempdir().unwrap();
    let settings = Settings {
        data_dir: dir.path().to_path_buf(),
        upload_dir: dir.path().join("uploads"),
        csv_dir: dir.path().join(".leo-studio"),
        ..Settings::default()
    };
    let mut store = Store::new(&settings).unwrap();
    let project = project_service::create_project(
        &mut store,
        &settings,
        "级联项目",
        "novel.txt",
        SAMPLE_NOVEL.as_bytes(),
        Some("text/plain"),
    )
    .unwrap();
    let request = CharacterCreateRequest {
        name: "林冲".into(),
        aliases: vec!["豹子头".into()],
        gender: None,
        age: None,
        role_type: None,
        status: None,
        description: None,
        tags: vec![],
    };
    let character =
        character_service::create_character(&mut store, &project.project_id, &request).unwrap();

    project_service::delete_project(&mut store, &project.project_id).unwrap();
    project_service::restore_project(&mut store, &project.project_id).unwrap();
    assert!(
        character_service::get_character(&store, &project.project_id, &character.character_id)
            .is_ok()
    );

    project_service::delete_project(&mut store, &project.project_id).unwrap();
    project_service::hard_delete_project(&mut store, &project.project_id).unwrap();
    assert!(store.characters.find_all().is_empty());
}

#[tokio::test]
async fn global_and_legacy_character_routes_return_404() {
    let app = TestApp::new(1).await;

    for path in [
        "/api/v3/characters",
        "/api/v3/characters/missing",
        "/api/v2/projects/p/characters",
        "/api/v1/projects/p/characters",
    ] {
        let (status, _) = app.get(path).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "path={path}");
    }

    let (status, _) = send_json(
        &app,
        "POST",
        "/api/v3/characters",
        serde_json::json!({"name": "全局角色"}),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
