use rmcp::model::{ReadResourceResult, Resource, ResourceContents, ResourceTemplate};
use serde_json::json;

use leo_plugin_api::error::AppError;
use leo_plugin_api::modules::projects::characters::service::{
    self as character_service, CharacterListFilter,
};
use leo_plugin_api::modules::projects::dialogues::service::{
    self as dialogue_service, DialogueListFilter,
};
use leo_plugin_api::modules::projects::service as project_service;

use super::server::McpHandler;

fn resource(uri: &str, name: &str, description: &str, mime_type: &str) -> Resource {
    let mut item = Resource::new(uri, name);
    item.description = Some(description.to_string());
    item.mime_type = Some(mime_type.to_string());
    item
}

/// 静态资源清单。
pub fn list() -> Vec<Resource> {
    vec![
        resource(
            "leo://projects",
            "projects",
            "全部未删除的剧目列表（最多 100 条）。",
            "application/json",
        ),
        resource(
            "leo://stats",
            "stats",
            "全库统计：剧目数、章节数、总字数、回收站数量。",
            "application/json",
        ),
    ]
}

/// 参数化资源模板。
pub fn templates() -> Vec<ResourceTemplate> {
    vec![
        ResourceTemplate::new("leo://projects/{project_id}", "project")
            .with_mime_type("application/json"),
        ResourceTemplate::new("leo://projects/{project_id}/chapters", "project-chapters")
            .with_mime_type("application/json"),
        ResourceTemplate::new(
            "leo://projects/{project_id}/characters",
            "project-characters",
        )
        .with_mime_type("application/json"),
        ResourceTemplate::new(
            "leo://projects/{project_id}/characters/{character_id}",
            "character",
        )
        .with_mime_type("application/json"),
        ResourceTemplate::new(
            "leo://projects/{project_id}/chapters/{chapter_id}",
            "chapter",
        )
        .with_mime_type("application/json"),
        ResourceTemplate::new(
            "leo://projects/{project_id}/chapters/{chapter_id}/text",
            "chapter-text",
        )
        .with_mime_type("text/plain; charset=utf-8"),
        ResourceTemplate::new(
            "leo://projects/{project_id}/chapters/{chapter_id}/dialogues",
            "chapter-dialogues",
        )
        .with_mime_type("application/json"),
        ResourceTemplate::new(
            "leo://projects/{project_id}/chapters/{chapter_id}/dialogues/{dialogue_id}",
            "dialogue",
        )
        .with_mime_type("application/json"),
    ]
}

fn contents(body: String, uri: &str, mime_type: &str) -> ReadResourceResult {
    ReadResourceResult::new(vec![
        ResourceContents::text(body, uri).with_mime_type(mime_type),
    ])
}

fn truncate(text: &str, max_chars: usize) -> (String, bool) {
    if text.chars().count() <= max_chars {
        (text.to_string(), false)
    } else {
        (text.chars().take(max_chars).collect(), true)
    }
}

/// 读取 `leo://` 资源。
pub async fn read(handler: &McpHandler, uri: &str) -> Result<ReadResourceResult, AppError> {
    let rest = uri
        .strip_prefix("leo://")
        .ok_or_else(|| AppError::invalid_parameter(format!("不支持的资源 URI：{uri}")))?;
    let segments: Vec<&str> = rest.split('/').filter(|item| !item.is_empty()).collect();

    match segments.as_slice() {
        ["projects"] => {
            let (items, total) = project_service::query_projects(
                &*handler.store.lock().await,
                1,
                100,
                false,
                None,
                None,
                "updated_at",
                "desc",
            );
            let body = serde_json::to_string_pretty(&json!({
                "total": total,
                "returned": items.len(),
                "items": items
            }))
            .map_err(|err| AppError::internal(format!("序列化资源失败：{err}")))?;
            Ok(contents(body, uri, "application/json"))
        }
        ["stats"] => {
            let stats = project_service::get_stats(&*handler.store.lock().await);
            let body = serde_json::to_string_pretty(&stats)
                .map_err(|err| AppError::internal(format!("序列化资源失败：{err}")))?;
            Ok(contents(body, uri, "application/json"))
        }
        ["projects", project_id] => {
            let info =
                project_service::get_project(&*handler.store.lock().await, project_id, false)?;
            let body = serde_json::to_string_pretty(&info)
                .map_err(|err| AppError::internal(format!("序列化资源失败：{err}")))?;
            Ok(contents(body, uri, "application/json"))
        }
        ["projects", project_id, "chapters"] => {
            let items =
                project_service::get_project_chapters(&*handler.store.lock().await, project_id)?;
            let body = serde_json::to_string_pretty(&json!({
                "project_id": project_id,
                "total": items.len(),
                "items": items
            }))
            .map_err(|err| AppError::internal(format!("序列化资源失败：{err}")))?;
            Ok(contents(body, uri, "application/json"))
        }
        ["projects", project_id, "characters"] => {
            let filter = CharacterListFilter {
                keyword: None,
                gender: None,
                role_type: None,
                status: None,
                tag: None,
            };
            let (items, total) = character_service::list_characters(
                &*handler.store.lock().await,
                project_id,
                1,
                100,
                "created_at",
                "desc",
                &filter,
            )?;
            let body = serde_json::to_string_pretty(&json!({
                "project_id": project_id,
                "total": total,
                "returned": items.len(),
                "items": items
            }))
            .map_err(|err| AppError::internal(format!("序列化资源失败：{err}")))?;
            Ok(contents(body, uri, "application/json"))
        }
        ["projects", project_id, "characters", character_id] => {
            let info = character_service::get_character(
                &*handler.store.lock().await,
                project_id,
                character_id,
            )?;
            let body = serde_json::to_string_pretty(&info)
                .map_err(|err| AppError::internal(format!("序列化资源失败：{err}")))?;
            Ok(contents(body, uri, "application/json"))
        }
        ["projects", project_id, "chapters", chapter_id] => {
            let info = project_service::get_project_chapter_content(
                &*handler.store.lock().await,
                project_id,
                chapter_id,
            )?;
            let (content, truncated) = truncate(&info.content, handler.mcp.max_chars);
            let body = serde_json::to_string_pretty(&json!({
                "project_id": project_id,
                "chapter_id": chapter_id,
                "chapter_index": info.chapter_index,
                "chapter_title": info.chapter_title,
                "word_count": info.word_count,
                "content": content,
                "truncated": truncated,
                "sentence_count": info.sentences.len()
            }))
            .map_err(|err| AppError::internal(format!("序列化资源失败：{err}")))?;
            Ok(contents(body, uri, "application/json"))
        }
        ["projects", project_id, "chapters", chapter_id, "dialogues"] => {
            let filter = DialogueListFilter {
                segment_type: None,
                binding_status: None,
                speaker_id: None,
                keyword: None,
            };
            let (items, total) = dialogue_service::list_dialogues(
                &*handler.store.lock().await,
                project_id,
                chapter_id,
                1,
                100,
                &filter,
            )?;
            let body = serde_json::to_string_pretty(&json!({
                "project_id": project_id,
                "chapter_id": chapter_id,
                "total": total,
                "returned": items.len(),
                "items": items
            }))
            .map_err(|err| AppError::internal(format!("序列化资源失败：{err}")))?;
            Ok(contents(body, uri, "application/json"))
        }
        [
            "projects",
            project_id,
            "chapters",
            chapter_id,
            "dialogues",
            dialogue_id,
        ] => {
            let dialogue = dialogue_service::get_dialogue(
                &*handler.store.lock().await,
                project_id,
                chapter_id,
                dialogue_id,
            )?;
            let body = serde_json::to_string_pretty(&dialogue)
                .map_err(|err| AppError::internal(format!("序列化资源失败：{err}")))?;
            Ok(contents(body, uri, "application/json"))
        }
        ["projects", project_id, "chapters", chapter_id, "text"] => {
            let info = project_service::get_project_chapter_content(
                &*handler.store.lock().await,
                project_id,
                chapter_id,
            )?;
            let (content, _) = truncate(&info.content, handler.mcp.max_chars);
            Ok(contents(content, uri, "text/plain; charset=utf-8"))
        }
        _ => Err(AppError::file_not_found(format!("资源不存在：{uri}"))),
    }
}
