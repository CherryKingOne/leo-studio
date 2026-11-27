use std::sync::Arc;

use leo_plugin_api::config::Settings;
use leo_plugin_api::state::AppState;
use leo_plugin_api::storage::store::Store;
use leo_plugin_mcp::{McpHandler, McpSettings};
use serde_json::{Map, Value, json};

const SAMPLE: &str = "第1章 开端\n林冲与武松相遇。";

fn handler() -> (tempfile::TempDir, McpHandler) {
    let dir = tempfile::tempdir().unwrap();
    let settings = Arc::new(Settings {
        data_dir: dir.path().to_path_buf(),
        upload_dir: dir.path().join("uploads"),
        csv_dir: dir.path().join(".leo-studio"),
        ..Settings::default()
    });
    let store = Store::new(&settings).unwrap();
    let mcp = Arc::new(McpSettings {
        enabled: true,
        http_path: "/mcp".into(),
        write_tools: true,
        destructive_tools: true,
        max_chars: 20_000,
        max_page_size: 100,
    });
    let state = AppState {
        settings,
        store: Arc::new(tokio::sync::Mutex::new(store)),
    };
    (dir, McpHandler::new(state, mcp))
}

async fn call(handler: &McpHandler, name: &str, args: Value) -> Value {
    let object: Map<String, Value> = args.as_object().unwrap().clone();
    handler.dispatch(name, &object).await.unwrap()
}

#[test]
fn tool_contract_has_explicit_schemas_and_canonical_parameters() {
    let (_dir, handler) = handler();
    let tools = handler.definitions();
    let names: Vec<&str> = tools.iter().map(|tool| tool.name.as_ref()).collect();

    assert!(names.contains(&"leo_create_character"));
    assert!(names.contains(&"leo_update_character"));
    assert!(names.contains(&"leo_delete_character"));
    assert!(names.contains(&"leo_delete_sentence_polyphone"));
    assert!(!names.contains(&"leo_rename_project"));
    assert!(tools.iter().all(|tool| tool.output_schema.is_some()));

    let sentence = tools
        .iter()
        .find(|tool| tool.name.as_ref() == "leo_get_sentence_polyphones")
        .unwrap();
    let required = sentence
        .input_schema
        .get("required")
        .unwrap()
        .as_array()
        .unwrap();
    assert!(required.iter().any(|value| value == "chapter_id"));

    let clear = tools
        .iter()
        .find(|tool| tool.name.as_ref() == "leo_clear_sentence_polyphones")
        .unwrap();
    assert_eq!(
        clear.annotations.as_ref().unwrap().destructive_hint,
        Some(true)
    );
    let required = clear
        .input_schema
        .get("required")
        .unwrap()
        .as_array()
        .unwrap();
    assert!(required.iter().any(|value| value == "confirm"));
}

#[tokio::test]
async fn character_tools_round_trip_and_project_isolation() {
    let (_dir, handler) = handler();
    let project = call(
        &handler,
        "leo_create_project",
        json!({ "name": "角色项目", "text": SAMPLE }),
    )
    .await;
    let project_id = project["project_id"].as_str().unwrap().to_string();

    let created = call(
        &handler,
        "leo_create_character",
        json!({
            "project_id": project_id,
            "name": "林冲",
            "aliases": ["豹子头"],
            "role_type": "protagonist"
        }),
    )
    .await;
    let character_id = created["character_id"].as_str().unwrap().to_string();

    let listed = call(
        &handler,
        "leo_list_characters",
        json!({ "project_id": project_id, "keyword": "林冲" }),
    )
    .await;
    assert_eq!(listed["total"], 1);

    let updated = call(
        &handler,
        "leo_update_character",
        json!({
            "project_id": project_id,
            "character_id": character_id,
            "status": "archived",
            "age": null
        }),
    )
    .await;
    assert_eq!(updated["status"], "archived");
    assert!(updated["age"].is_null());

    let deleted = call(
        &handler,
        "leo_delete_character",
        json!({
            "project_id": project_id,
            "character_id": character_id,
            "confirm": true
        }),
    )
    .await;
    assert_eq!(deleted["deleted"], true);

    let error = handler
        .dispatch(
            "leo_get_character",
            &json!({ "project_id": "missing", "character_id": character_id })
                .as_object()
                .unwrap()
                .clone(),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code, 40400);
}
