use std::path::{Path, PathBuf};
use std::sync::Arc;

use leo_app::config::Settings;
use leo_app::mcp::{McpHandler, McpSettings};
use leo_app::storage::store::Store;
use serde_json::{Map, Value, json};

const SAMPLE: &str = "第一章 开端\n小明重行长，银行行长不重。\n第二章 重逢\n行路人说：重行可以。\n";

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "leo-mcp-{name}-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("创建临时目录失败");
    dir
}

fn settings(dir: &Path) -> Settings {
    Settings {
        data_dir: dir.to_path_buf(),
        upload_dir: dir.join("storage/uploads/text-files"),
        csv_dir: dir.join(".leo-studio"),
        ..Settings::default()
    }
}

fn mcp_settings(write: bool, destructive: bool) -> McpSettings {
    McpSettings {
        enabled: true,
        http_path: "/mcp".to_string(),
        write_tools: write,
        destructive_tools: destructive,
        max_chars: 20_000,
        max_page_size: 100,
    }
}

async fn make_handler(dir: &Path, mcp: McpSettings) -> McpHandler {
    let settings = Arc::new(settings(dir));
    let store = Store::new(&settings).expect("初始化 Store 失败");
    let state = leo_app::build_app_state(settings, store)
        .await
        .expect("插件装配失败");
    McpHandler::new(state, Arc::new(mcp))
}

fn args(value: Value) -> Map<String, Value> {
    value.as_object().expect("参数必须是对象").clone()
}

async fn call(handler: &McpHandler, name: &str, value: Value) -> Value {
    handler
        .dispatch(name, &args(value))
        .await
        .unwrap_or_else(|err| panic!("调用 {name} 失败：{}", err.message))
}

async fn call_err(handler: &McpHandler, name: &str, value: Value) -> String {
    handler
        .dispatch(name, &args(value))
        .await
        .expect_err("预期返回错误")
        .message
}

#[tokio::test]
async fn read_only_tools_are_default() {
    let dir = temp_dir("tools-default");
    let handler = make_handler(&dir, mcp_settings(false, false)).await;
    let names: Vec<String> = handler
        .definitions()
        .into_iter()
        .map(|tool| tool.name.to_string())
        .collect();

    assert!(names.contains(&"leo_health".to_string()));
    assert!(names.contains(&"leo_get_chapter".to_string()));
    assert!(!names.contains(&"leo_create_project".to_string()));
    assert!(!names.contains(&"leo_purge_project".to_string()));
}

#[tokio::test]
async fn write_and_destructive_tools_are_gated() {
    let dir = temp_dir("tools-gated");
    let handler = make_handler(&dir, mcp_settings(true, false)).await;
    let names: Vec<String> = handler
        .definitions()
        .into_iter()
        .map(|tool| tool.name.to_string())
        .collect();
    assert!(names.contains(&"leo_create_project".to_string()));
    assert!(!names.contains(&"leo_purge_project".to_string()));

    let handler_all = make_handler(&dir, mcp_settings(true, true)).await;
    let names: Vec<String> = handler_all
        .definitions()
        .into_iter()
        .map(|tool| tool.name.to_string())
        .collect();
    assert!(names.contains(&"leo_purge_project".to_string()));
    assert!(names.contains(&"leo_empty_recycle_bin".to_string()));
}

#[tokio::test]
async fn create_query_search_and_stats() {
    let dir = temp_dir("create-query");
    let handler = make_handler(&dir, mcp_settings(true, true)).await;

    let project = call(
        &handler,
        "leo_create_project",
        json!({ "name": "MCP 测试剧目", "text": SAMPLE }),
    )
    .await;
    let project_id = project["project_id"]
        .as_str()
        .expect("project_id")
        .to_string();

    let list = call(&handler, "leo_list_projects", json!({ "keyword": "MCP" })).await;
    assert_eq!(list["total"], 1);

    let chapters = call(
        &handler,
        "leo_list_chapters",
        json!({ "project_id": project_id }),
    )
    .await;
    assert_eq!(chapters["total"], 2);

    let chapter_id = chapters["items"][0]["chapter_id"]
        .as_str()
        .expect("chapter_id")
        .to_string();
    let chapter = call(
        &handler,
        "leo_get_chapter",
        json!({ "project_id": project_id, "chapter_id": chapter_id, "max_chars": 5 }),
    )
    .await;
    assert!(chapter["content"].as_str().unwrap().chars().count() <= 5);
    assert_eq!(chapter["truncated"], true);

    let found = call(
        &handler,
        "leo_search_chapters",
        json!({ "project_id": project_id, "keyword": "银行" }),
    )
    .await;
    assert_eq!(found["total"], 1);

    let stats = call(&handler, "leo_get_stats", json!({})).await;
    assert_eq!(stats["projects"], 1);
    assert_eq!(stats["chapters"], 2);
}

#[tokio::test]
async fn append_is_dry_run_by_default_and_needs_confirm() {
    let dir = temp_dir("append");
    let handler = make_handler(&dir, mcp_settings(true, true)).await;

    let project = call(
        &handler,
        "leo_create_project",
        json!({ "name": "追更测试", "text": SAMPLE }),
    )
    .await;
    let project_id = project["project_id"].as_str().unwrap().to_string();

    let preview = call(
        &handler,
        "leo_append_chapters",
        json!({ "project_id": project_id, "text": SAMPLE }),
    )
    .await;
    assert_eq!(preview["dry_run"], true);
    assert_eq!(preview["added_chapters"], 0);
    assert!(preview["skipped_chapters"].as_i64().unwrap() >= 2);

    let message = call_err(
        &handler,
        "leo_append_chapters",
        json!({ "project_id": project_id, "text": SAMPLE, "dry_run": false }),
    )
    .await;
    assert!(message.contains("confirm"));

    let applied = call(
        &handler,
        "leo_append_chapters",
        json!({
            "project_id": project_id,
            "text": SAMPLE,
            "dry_run": false,
            "overwrite_existing": true,
            "confirm": true
        }),
    )
    .await;
    assert_eq!(applied["dry_run"], false);
}

#[tokio::test]
async fn polyphone_round_trip_and_destructive_confirm() {
    let dir = temp_dir("polyphone");
    let handler = make_handler(&dir, mcp_settings(true, true)).await;

    let project = call(
        &handler,
        "leo_create_project",
        json!({ "name": "多音字测试", "text": SAMPLE }),
    )
    .await;
    let project_id = project["project_id"].as_str().unwrap().to_string();
    let chapters = call(
        &handler,
        "leo_list_chapters",
        json!({ "project_id": project_id }),
    )
    .await;
    let chapter_id = chapters["items"][0]["chapter_id"]
        .as_str()
        .unwrap()
        .to_string();
    let chapter = call(
        &handler,
        "leo_get_chapter",
        json!({ "project_id": project_id, "chapter_id": chapter_id }),
    )
    .await;
    let sentence = &chapter["sentences"][0];
    let sentence_id = sentence["sentence_id"].as_str().unwrap().to_string();
    let character = sentence["content"]
        .as_str()
        .unwrap()
        .chars()
        .next()
        .unwrap()
        .to_string();

    let saved = call(
        &handler,
        "leo_save_sentence_polyphones",
        json!({
            "project_id": project_id,
            "sentence_id": sentence_id,
            "annotations": [{
                "char_index": 0,
                "character": character,
                "pinyin": "hang",
                "tone": 2,
                "tts_pinyin": "háng"
            }]
        }),
    )
    .await;
    assert_eq!(saved["total"], 1);

    let listed = call(
        &handler,
        "leo_get_sentence_polyphones",
        json!({ "project_id": project_id, "sentence_id": sentence_id }),
    )
    .await;
    assert_eq!(listed["items"][0]["pinyin"], "hang");

    let cleared = call(
        &handler,
        "leo_clear_sentence_polyphones",
        json!({ "project_id": project_id, "sentence_id": sentence_id }),
    )
    .await;
    assert!(cleared["deleted_count"].as_i64().unwrap() >= 1);

    let message = call_err(
        &handler,
        "leo_move_to_trash",
        json!({ "project_id": project_id }),
    )
    .await;
    assert!(message.contains("confirm"));

    let moved = call(
        &handler,
        "leo_move_to_trash",
        json!({ "project_id": project_id, "confirm": true }),
    )
    .await;
    assert_eq!(moved["project_id"], project_id);

    let recycle = call(&handler, "leo_list_recycle_bin", json!({})).await;
    assert_eq!(recycle["total"], 1);

    let restored = call(
        &handler,
        "leo_restore_project",
        json!({ "project_id": project_id }),
    )
    .await;
    assert_eq!(restored["project_id"], project_id);
}

#[tokio::test]
async fn unknown_tool_is_rejected() {
    let dir = temp_dir("unknown");
    let handler = make_handler(&dir, mcp_settings(true, true)).await;
    let message = call_err(&handler, "leo_not_exists", json!({})).await;
    assert!(message.contains("未知工具"));
}

#[tokio::test]
async fn chapter_update_and_delete() {
    let dir = temp_dir("chapter-crud");
    let handler = make_handler(&dir, mcp_settings(true, true)).await;

    let project = call(
        &handler,
        "leo_create_project",
        json!({ "name": "章节改删测试", "text": SAMPLE }),
    )
    .await;
    let project_id = project["project_id"].as_str().unwrap().to_string();
    let chapters = call(
        &handler,
        "leo_list_chapters",
        json!({ "project_id": project_id }),
    )
    .await;
    let chapter_id = chapters["items"][0]["chapter_id"]
        .as_str()
        .unwrap()
        .to_string();

    let renamed = call(
        &handler,
        "leo_update_chapter",
        json!({
            "project_id": project_id,
            "chapter_id": chapter_id,
            "title": "第一章 改名"
        }),
    )
    .await;
    assert_eq!(renamed["chapter_title"], "第一章 改名");

    let message = call_err(
        &handler,
        "leo_update_chapter",
        json!({
            "project_id": project_id,
            "chapter_id": chapter_id,
            "content": "新的正文内容。"
        }),
    )
    .await;
    assert!(message.contains("confirm"));

    let updated = call(
        &handler,
        "leo_update_chapter",
        json!({
            "project_id": project_id,
            "chapter_id": chapter_id,
            "content": "新的正文内容。",
            "confirm": true
        }),
    )
    .await;
    assert_eq!(updated["content"], "新的正文内容。");
    assert!(!updated["sentences"].as_array().unwrap().is_empty());

    let deleted = call(
        &handler,
        "leo_delete_chapter",
        json!({
            "project_id": project_id,
            "chapter_id": chapter_id,
            "confirm": true
        }),
    )
    .await;
    assert_eq!(deleted["deleted"], true);

    let chapters = call(
        &handler,
        "leo_list_chapters",
        json!({ "project_id": project_id }),
    )
    .await;
    assert_eq!(chapters["total"], 1);
}

#[tokio::test]
async fn api_and_mcp_plugins_go_through_unified_install() {
    use leo_app::build_app_state_with;
    use leo_app::mcp::McpPlugin;

    let dir = temp_dir("plugin-install");
    let settings = Arc::new(settings(&dir));
    let store = Store::new(&settings).expect("初始化 Store 失败");
    let mcp = Arc::new(McpPlugin::new(Arc::new(mcp_settings(false, false))));

    let state = build_app_state_with(settings, store, vec![mcp])
        .await
        .expect("插件装配失败");
    let loaded = state.plugin_manager.loaded_ids().await;
    let api_at = loaded.iter().position(|id| id == "leo-plugin-api");
    let mcp_at = loaded.iter().position(|id| id == "leo-plugin-mcp");
    assert!(api_at.is_some(), "ApiPlugin::apply 未执行: {loaded:?}");
    assert!(mcp_at.is_some(), "McpPlugin::apply 未执行: {loaded:?}");
    assert!(api_at < mcp_at, "McpPlugin 依赖 ApiPlugin，应在其后挂载");
    assert!(state.plugin_manager.failures().await.is_empty());
}

#[tokio::test]
async fn mcp_plugin_without_api_plugin_fails_loudly() {
    use leo_app::mcp::McpPlugin;
    use leo_core::plugin::{Plugin, PluginManager};

    let manager = PluginManager::new();
    let mcp: Arc<dyn Plugin> = Arc::new(McpPlugin::new(Arc::new(mcp_settings(false, false))));
    manager.register(mcp).await.unwrap();
    let err = manager
        .apply_all(&leo_core::Context::new("mcp_only"))
        .await
        .expect_err("缺少 leo-plugin-api 时应报错");
    assert!(err.to_string().contains("leo-plugin-api"), "{err}");
}
