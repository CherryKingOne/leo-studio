use rmcp::model::{JsonObject, Tool};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use leo_plugin_api::error::AppError;
use leo_plugin_api::http::page_count;
use leo_plugin_api::modules::projects::characters::schema::{
    CharacterCreateRequest, CharacterListResponse, CharacterUpdateRequest,
};
use leo_plugin_api::modules::projects::characters::service::{
    self as character_service, CharacterListFilter,
};
use leo_plugin_api::modules::projects::dialogues::schema::{
    DialogueListResponse, DialogueSpeakerBindRequest,
};
use leo_plugin_api::modules::projects::dialogues::service::{
    self as dialogue_service, DialogueListFilter as DialogueFilter,
};
use leo_plugin_api::modules::projects::model::ChapterContentInfo;
use leo_plugin_api::modules::projects::schema::PolyphoneAnnotation;
use leo_plugin_api::modules::projects::service as project_service;

use super::server::McpHandler;

pub use super::tools::definitions;

fn value_of<T: serde::Serialize>(value: &T) -> Result<Value, AppError> {
    serde_json::to_value(value).map_err(|err| AppError::internal(format!("序列化响应失败：{err}")))
}

fn parse_payload<T: DeserializeOwned>(args: &JsonObject) -> Result<T, AppError> {
    serde_json::from_value(Value::Object(args.clone()))
        .map_err(|err| AppError::invalid_parameter(format!("参数解析失败：{err}")))
}

fn arg<'a>(args: &'a JsonObject, key: &str) -> Option<&'a Value> {
    args.get(key)
}

fn req_string(args: &JsonObject, key: &str) -> Result<String, AppError> {
    let value = arg(args, key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| AppError::invalid_parameter(format!("缺少参数：{key}")))?;
    Ok(value.to_string())
}

fn opt_string(args: &JsonObject, key: &str) -> Option<String> {
    arg(args, key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn opt_i64(args: &JsonObject, key: &str, default: i64) -> i64 {
    arg(args, key).and_then(Value::as_i64).unwrap_or(default)
}

fn opt_bool(args: &JsonObject, key: &str, default: bool) -> bool {
    arg(args, key).and_then(Value::as_bool).unwrap_or(default)
}

fn pagination(args: &JsonObject, max_page_size: usize) -> Result<(usize, usize), AppError> {
    let page = opt_i64(args, "page", 1);
    let page_size = opt_i64(args, "page_size", 20);
    if page < 1 {
        return Err(AppError::invalid_parameter("页码必须大于 0"));
    }
    if page_size < 1 || page_size as usize > max_page_size {
        return Err(AppError::invalid_parameter(format!(
            "每页数量必须在 1 到 {max_page_size} 之间"
        )));
    }
    Ok((page as usize, page_size as usize))
}

fn require_confirm(args: &JsonObject, action: &str) -> Result<(), AppError> {
    if opt_bool(args, "confirm", false) {
        Ok(())
    } else {
        Err(AppError::invalid_parameter(format!(
            "{action} 属于破坏性操作，请显式传入 confirm=true"
        )))
    }
}

fn validate_enum(
    value: Option<&str>,
    allowed: &[&str],
    field: &str,
) -> Result<Option<String>, AppError> {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    if !allowed.contains(&value) {
        return Err(AppError::invalid_parameter(format!(
            "{field}不支持 {value}，可选值：{}",
            allowed.join(", ")
        )));
    }
    Ok(Some(value.to_string()))
}

fn truncate_text(text: &str, max_chars: usize) -> (String, bool) {
    if text.chars().count() <= max_chars {
        (text.to_string(), false)
    } else {
        (text.chars().take(max_chars).collect(), true)
    }
}

fn truncate_chapter(chapter: &mut ChapterContentInfo, max_chars: usize) -> bool {
    let mut truncated = false;
    let (content, cut) = truncate_text(&chapter.content, max_chars);
    chapter.content = content;
    truncated |= cut;
    for sentence in &mut chapter.sentences {
        let (text, cut) = truncate_text(&sentence.content, max_chars);
        sentence.content = text;
        truncated |= cut;
    }
    truncated
}

impl McpHandler {
    pub fn definitions(&self) -> Vec<Tool> {
        (*self.tools).clone()
    }

    pub fn has_tool(&self, name: &str) -> bool {
        self.tools.iter().any(|tool| tool.name.as_ref() == name)
    }

    fn read_text_input(&self, args: &JsonObject) -> Result<(String, String), AppError> {
        let text = req_string(args, "text")?;
        Ok((text, "model.txt".to_string()))
    }

    pub async fn dispatch(&self, name: &str, args: &JsonObject) -> Result<Value, AppError> {
        match name {
            "leo_health" => Ok(json!({
                "status": "ok",
                "server": "leo-studio",
                "app": self.settings.app_name,
                "version": self.settings.app_version,
                "protocol_versions": ["2026-07-28", "2025-11-25", "2025-06-18"],
                "write_tools": self.mcp.write_tools,
                "destructive_tools": self.mcp.destructive_tools
            })),
            "leo_list_projects" => {
                let (page, page_size) = pagination(args, self.mcp.max_page_size)?;
                let (items, total) = project_service::query_projects(
                    &*self.store.lock().await,
                    page,
                    page_size,
                    false,
                    opt_string(args, "status").as_deref(),
                    opt_string(args, "keyword").as_deref(),
                    &opt_string(args, "sort").unwrap_or_else(|| "created_at".to_string()),
                    &opt_string(args, "order").unwrap_or_else(|| "desc".to_string()),
                );
                Ok(json!({
                    "total": total,
                    "page": page,
                    "page_size": page_size,
                    "pages": page_count(total, page_size),
                    "items": items
                }))
            }
            "leo_get_project" => {
                let project_id = req_string(args, "project_id")?;
                let info =
                    project_service::get_project(&*self.store.lock().await, &project_id, false)?;
                Ok(value_of(&info)?)
            }
            "leo_list_chapters" => {
                let project_id = req_string(args, "project_id")?;
                let mut items =
                    project_service::get_project_chapters(&*self.store.lock().await, &project_id)?;
                if let Some(keyword) = opt_string(args, "keyword") {
                    items.retain(|item| item.chapter_title.contains(&keyword));
                }
                let total = items.len();
                if let Ok(limit) = opt_i64(args, "limit", i64::MAX).try_into() {
                    items.truncate(limit);
                }
                Ok(json!({
                    "project_id": project_id,
                    "total": total,
                    "items": items
                }))
            }
            "leo_get_chapter" => {
                let project_id = req_string(args, "project_id")?;
                let chapter_id = req_string(args, "chapter_id")?;
                let max_chars =
                    opt_i64(args, "max_chars", self.mcp.max_chars as i64).max(1) as usize;
                let include_sentences = opt_bool(args, "include_sentences", true);
                let mut info = project_service::get_project_chapter_content(
                    &*self.store.lock().await,
                    &project_id,
                    &chapter_id,
                )?;
                let truncated = truncate_chapter(&mut info, max_chars);
                if !include_sentences {
                    info.sentences.clear();
                }
                let mut value = value_of(&info)?;
                if let Some(object) = value.as_object_mut() {
                    object.insert("truncated".to_string(), json!(truncated));
                }
                Ok(value)
            }
            "leo_search_chapters" => {
                let project_id = req_string(args, "project_id")?;
                let keyword = req_string(args, "keyword")?;
                let limit = opt_i64(args, "limit", 50).max(1) as usize;
                let items = project_service::search_chapters(
                    &*self.store.lock().await,
                    &project_id,
                    &keyword,
                    limit,
                )?;
                Ok(json!({ "project_id": project_id, "total": items.len(), "items": items }))
            }
            "leo_list_dialogues" | "leo_list_unbound_dialogues" => {
                let project_id = req_string(args, "project_id")?;
                let chapter_id = req_string(args, "chapter_id")?;
                let (page, page_size) = pagination(args, self.mcp.max_page_size)?;
                let segment_type = validate_enum(
                    opt_string(args, "segment_type").as_deref(),
                    &["narration", "dialogue"],
                    "片段类型",
                )?;
                let binding_status = if name == "leo_list_unbound_dialogues" {
                    Some("unbound".to_string())
                } else {
                    validate_enum(
                        opt_string(args, "binding_status").as_deref(),
                        &["all", "bound", "unbound"],
                        "绑定状态",
                    )?
                };
                let speaker_id = opt_string(args, "speaker_id");
                let keyword = opt_string(args, "keyword");
                let filter = DialogueFilter {
                    segment_type: segment_type.as_deref(),
                    binding_status: binding_status.as_deref(),
                    speaker_id: speaker_id.as_deref(),
                    keyword: keyword.as_deref(),
                };
                let (items, total) = dialogue_service::list_dialogues(
                    &*self.store.lock().await,
                    &project_id,
                    &chapter_id,
                    page,
                    page_size,
                    &filter,
                )?;
                Ok(value_of(&DialogueListResponse {
                    project_id,
                    chapter_id,
                    total,
                    page,
                    page_size,
                    pages: page_count(total, page_size),
                    items,
                })?)
            }
            "leo_get_dialogue" => {
                let project_id = req_string(args, "project_id")?;
                let chapter_id = req_string(args, "chapter_id")?;
                let dialogue_id = req_string(args, "dialogue_id")?;
                let dialogue = dialogue_service::get_dialogue(
                    &*self.store.lock().await,
                    &project_id,
                    &chapter_id,
                    &dialogue_id,
                )?;
                Ok(value_of(&dialogue)?)
            }
            "leo_list_recycle_bin" => {
                let (page, page_size) = pagination(args, self.mcp.max_page_size)?;
                let (items, total) = project_service::query_projects(
                    &*self.store.lock().await,
                    page,
                    page_size,
                    true,
                    None,
                    None,
                    "created_at",
                    "desc",
                );
                Ok(json!({
                    "total": total,
                    "page": page,
                    "page_size": page_size,
                    "pages": page_count(total, page_size),
                    "items": items
                }))
            }
            "leo_get_sentence_polyphones" => {
                let project_id = req_string(args, "project_id")?;
                let chapter_id = req_string(args, "chapter_id")?;
                let sentence_id = req_string(args, "sentence_id")?;
                let items = project_service::get_chapter_sentence_polyphones(
                    &*self.store.lock().await,
                    &project_id,
                    &chapter_id,
                    &sentence_id,
                )?;
                Ok(json!({
                    "sentence_id": sentence_id,
                    "total": items.len(),
                    "items": items
                }))
            }
            "leo_list_chapter_polyphones" => {
                let project_id = req_string(args, "project_id")?;
                let chapter_id = req_string(args, "chapter_id")?;
                let info = project_service::get_chapter_polyphones(
                    &*self.store.lock().await,
                    &project_id,
                    &chapter_id,
                )?;
                Ok(value_of(&info)?)
            }
            "leo_get_stats" => {
                let stats = project_service::get_stats(&*self.store.lock().await);
                Ok(value_of(&stats)?)
            }
            "leo_create_project" => {
                let name = req_string(args, "name")?;
                let (text, filename) = self.read_text_input(args)?;
                let info = project_service::create_project(
                    &mut *self.store.lock().await,
                    &self.settings,
                    &name,
                    &filename,
                    text.as_bytes(),
                    None,
                )?;
                Ok(value_of(&info)?)
            }
            "leo_append_chapters" => {
                let project_id = req_string(args, "project_id")?;
                let (text, filename) = self.read_text_input(args)?;
                let overwrite_existing = opt_bool(args, "overwrite_existing", false);
                let dry_run = opt_bool(args, "dry_run", true);
                if !dry_run {
                    require_confirm(args, "追更写入")?;
                }
                let info = project_service::append_chapters(
                    &mut *self.store.lock().await,
                    &self.settings,
                    &project_id,
                    &filename,
                    text.as_bytes(),
                    None,
                    overwrite_existing,
                    dry_run,
                )?;
                Ok(value_of(&info)?)
            }
            "leo_update_project" => {
                let project_id = req_string(args, "project_id")?;
                let info = project_service::update_project_fields(
                    &mut *self.store.lock().await,
                    &project_id,
                    opt_string(args, "name").as_deref(),
                    opt_string(args, "status").as_deref(),
                )?;
                Ok(value_of(&info)?)
            }
            "leo_update_chapter" => {
                let project_id = req_string(args, "project_id")?;
                let chapter_id = req_string(args, "chapter_id")?;
                let title = opt_string(args, "title");
                let content = opt_string(args, "content");
                if title.is_none() && content.is_none() {
                    return Err(AppError::invalid_parameter("请至少提供 title 或 content"));
                }
                if content.is_some() {
                    require_confirm(args, "覆盖章节正文")?;
                }
                let info = project_service::update_chapter(
                    &mut *self.store.lock().await,
                    &project_id,
                    &chapter_id,
                    title.as_deref(),
                    content.as_deref(),
                )?;
                Ok(value_of(&info)?)
            }
            "leo_save_sentence_polyphones" => {
                let project_id = req_string(args, "project_id")?;
                let chapter_id = req_string(args, "chapter_id")?;
                let sentence_id = req_string(args, "sentence_id")?;
                let raw = arg(args, "annotations")
                    .and_then(Value::as_array)
                    .ok_or_else(|| AppError::invalid_parameter("缺少参数：annotations"))?;
                let mut annotations_list = Vec::with_capacity(raw.len());
                for item in raw {
                    let object = item
                        .as_object()
                        .ok_or_else(|| AppError::invalid_parameter("annotations 必须是对象数组"))?;
                    annotations_list.push(PolyphoneAnnotation {
                        char_index: object
                            .get("char_index")
                            .and_then(Value::as_i64)
                            .unwrap_or_default(),
                        character: object
                            .get("character")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_string(),
                        pinyin: object
                            .get("pinyin")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_string(),
                        pinyin_base: object
                            .get("pinyin_base")
                            .and_then(Value::as_str)
                            .map(str::to_string),
                        tone: object.get("tone").and_then(Value::as_i64),
                        tts_pinyin: object
                            .get("tts_pinyin")
                            .and_then(Value::as_str)
                            .map(str::to_string),
                    });
                }
                let items = project_service::save_chapter_sentence_polyphones(
                    &mut *self.store.lock().await,
                    &project_id,
                    &chapter_id,
                    &sentence_id,
                    &annotations_list,
                )?;
                Ok(json!({
                    "sentence_id": sentence_id,
                    "total": items.len(),
                    "items": items
                }))
            }
            "leo_clear_sentence_polyphones" => {
                require_confirm(args, "清空多音字标注")?;
                let project_id = req_string(args, "project_id")?;
                let chapter_id = req_string(args, "chapter_id")?;
                let sentence_id = req_string(args, "sentence_id")?;
                let deleted_count = project_service::delete_chapter_sentence_polyphones(
                    &mut *self.store.lock().await,
                    &project_id,
                    &chapter_id,
                    &sentence_id,
                )?;
                Ok(json!({
                    "sentence_id": sentence_id,
                    "deleted_count": deleted_count,
                    "deleted": true
                }))
            }
            "leo_delete_sentence_polyphone" => {
                require_confirm(args, "删除多音字标注")?;
                let project_id = req_string(args, "project_id")?;
                let chapter_id = req_string(args, "chapter_id")?;
                let sentence_id = req_string(args, "sentence_id")?;
                let char_index = opt_i64(args, "char_index", -1);
                let info = project_service::delete_chapter_sentence_polyphone(
                    &mut *self.store.lock().await,
                    &project_id,
                    &chapter_id,
                    &sentence_id,
                    char_index,
                )?;
                Ok(value_of(&info)?)
            }
            "leo_restore_project" => {
                let project_id = req_string(args, "project_id")?;
                let info =
                    project_service::restore_project(&mut *self.store.lock().await, &project_id)?;
                Ok(value_of(&info)?)
            }
            "leo_delete_chapter" => {
                require_confirm(args, "删除章节")?;
                let project_id = req_string(args, "project_id")?;
                let chapter_id = req_string(args, "chapter_id")?;
                let info = project_service::delete_chapter(
                    &mut *self.store.lock().await,
                    &project_id,
                    &chapter_id,
                )?;
                Ok(value_of(&info)?)
            }
            "leo_move_to_trash" => {
                require_confirm(args, "移入回收站")?;
                let project_id = req_string(args, "project_id")?;
                let info =
                    project_service::delete_project(&mut *self.store.lock().await, &project_id)?;
                Ok(value_of(&info)?)
            }
            "leo_purge_project" => {
                require_confirm(args, "彻底删除剧目")?;
                let project_id = req_string(args, "project_id")?;
                let info = project_service::hard_delete_project(
                    &mut *self.store.lock().await,
                    &project_id,
                )?;
                Ok(value_of(&info)?)
            }
            "leo_empty_recycle_bin" => {
                require_confirm(args, "清空回收站")?;
                let cleared_count =
                    project_service::clear_recycle_bin(&mut *self.store.lock().await)?;
                Ok(json!({ "cleared_count": cleared_count, "cleared": true }))
            }
            "leo_list_characters" => {
                let project_id = req_string(args, "project_id")?;
                let (page, page_size) = pagination(args, self.mcp.max_page_size)?;
                let gender = validate_enum(
                    opt_string(args, "gender").as_deref(),
                    &["male", "female", "unknown", "other"],
                    "性别",
                )?;
                let role_type = validate_enum(
                    opt_string(args, "role_type").as_deref(),
                    &[
                        "protagonist",
                        "supporting",
                        "antagonist",
                        "narrator",
                        "other",
                    ],
                    "角色类型",
                )?;
                let status = match opt_string(args, "status").as_deref() {
                    None => Some("active".to_string()),
                    Some("all") => None,
                    Some("active") => Some("active".to_string()),
                    Some("archived") => Some("archived".to_string()),
                    Some(value) => {
                        return Err(AppError::invalid_parameter(format!(
                            "角色状态不支持 {value}，可选值：active, archived, all"
                        )));
                    }
                };
                let keyword = opt_string(args, "keyword");
                let tag = opt_string(args, "tag");
                let filter = CharacterListFilter {
                    keyword: keyword.as_deref(),
                    gender: gender.as_deref(),
                    role_type: role_type.as_deref(),
                    status: status.as_deref(),
                    tag: tag.as_deref(),
                };
                let (items, total) = character_service::list_characters(
                    &*self.store.lock().await,
                    &project_id,
                    page,
                    page_size,
                    &opt_string(args, "sort").unwrap_or_else(|| "created_at".to_string()),
                    &opt_string(args, "order").unwrap_or_else(|| "desc".to_string()),
                    &filter,
                )?;
                Ok(value_of(&CharacterListResponse {
                    project_id,
                    total,
                    page,
                    page_size,
                    pages: page_count(total, page_size),
                    items,
                })?)
            }
            "leo_get_character" => {
                let project_id = req_string(args, "project_id")?;
                let character_id = req_string(args, "character_id")?;
                let character = character_service::get_character(
                    &*self.store.lock().await,
                    &project_id,
                    &character_id,
                )?;
                Ok(value_of(&character)?)
            }
            "leo_bind_dialogue_speaker" => {
                let project_id = req_string(args, "project_id")?;
                let chapter_id = req_string(args, "chapter_id")?;
                let dialogue_id = req_string(args, "dialogue_id")?;
                let payload: DialogueSpeakerBindRequest = parse_payload(args)?;
                let dialogue = dialogue_service::bind_speaker(
                    &mut *self.store.lock().await,
                    &project_id,
                    &chapter_id,
                    &dialogue_id,
                    &payload,
                )?;
                Ok(value_of(&dialogue)?)
            }
            "leo_unbind_dialogue_speaker" => {
                let project_id = req_string(args, "project_id")?;
                let chapter_id = req_string(args, "chapter_id")?;
                let dialogue_id = req_string(args, "dialogue_id")?;
                let result = dialogue_service::unbind_speaker(
                    &mut *self.store.lock().await,
                    &project_id,
                    &chapter_id,
                    &dialogue_id,
                )?;
                Ok(value_of(&result)?)
            }
            "leo_create_character" => {
                let project_id = req_string(args, "project_id")?;
                let payload: CharacterCreateRequest = parse_payload(args)?;
                let character = character_service::create_character(
                    &mut *self.store.lock().await,
                    &project_id,
                    &payload,
                )?;
                Ok(value_of(&character)?)
            }
            "leo_update_character" => {
                let project_id = req_string(args, "project_id")?;
                let character_id = req_string(args, "character_id")?;
                let payload: CharacterUpdateRequest = parse_payload(args)?;
                let character = character_service::update_character(
                    &mut *self.store.lock().await,
                    &project_id,
                    &character_id,
                    &payload,
                )?;
                Ok(value_of(&character)?)
            }
            "leo_delete_character" => {
                require_confirm(args, "删除角色")?;
                let project_id = req_string(args, "project_id")?;
                let character_id = req_string(args, "character_id")?;
                let result = character_service::delete_character(
                    &mut *self.store.lock().await,
                    &project_id,
                    &character_id,
                )?;
                Ok(value_of(&result)?)
            }
            _ => Err(AppError::invalid_parameter(format!("未知工具：{name}"))),
        }
    }
}
