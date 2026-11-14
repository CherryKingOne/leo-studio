use std::path::{Path, PathBuf};
use std::sync::Arc;

use rmcp::model::{JsonObject, Tool, ToolAnnotations};
use serde_json::{Map, Value, json};

use crate::error::AppError;
use crate::http::page_count;
use crate::modules::projects::model::ChapterContentInfo;
use crate::modules::projects::schema::PolyphoneAnnotation;
use crate::modules::projects::service as project_service;

use super::config::McpSettings;
use super::server::McpHandler;

fn str_prop(description: &str) -> Value {
    json!({ "type": "string", "description": description })
}

fn int_prop(description: &str) -> Value {
    json!({ "type": "integer", "description": description })
}

fn bool_prop(description: &str) -> Value {
    json!({ "type": "boolean", "description": description })
}

fn enum_prop(values: &[&str], description: &str) -> Value {
    json!({ "type": "string", "enum": values, "description": description })
}

fn object_schema(properties: Value, required: &[&str]) -> Arc<JsonObject> {
    let mut map = Map::new();
    map.insert("type".to_string(), json!("object"));
    map.insert("properties".to_string(), properties);
    map.insert("required".to_string(), json!(required));
    map.insert("additionalProperties".to_string(), json!(false));
    Arc::new(map)
}

fn output_schema() -> Arc<JsonObject> {
    let mut map = Map::new();
    map.insert("type".to_string(), json!("object"));
    map.insert("additionalProperties".to_string(), json!(true));
    Arc::new(map)
}

fn annotations(
    title: &str,
    read_only: bool,
    destructive: bool,
    idempotent: bool,
) -> ToolAnnotations {
    let mut item = ToolAnnotations::default();
    item.title = Some(title.to_string());
    item.read_only_hint = Some(read_only);
    item.destructive_hint = Some(destructive);
    item.idempotent_hint = Some(idempotent);
    item.open_world_hint = Some(false);
    item
}

#[allow(clippy::too_many_arguments)]
fn tool(
    name: &'static str,
    title: &'static str,
    description: &'static str,
    properties: Value,
    required: &[&str],
    tool_annotations: ToolAnnotations,
) -> Tool {
    let mut item = Tool::default();
    item.name = name.into();
    item.title = Some(title.to_string());
    item.description = Some(description.into());
    item.input_schema = object_schema(properties, required);
    item.output_schema = Some(output_schema());
    item.annotations = Some(tool_annotations);
    item
}

/// 构建当前配置下对外暴露的工具清单。
pub fn definitions(settings: &McpSettings) -> Vec<Tool> {
    let mut tools = vec![
        tool(
            "leo_health",
            "服务健康",
            "检查 Leo Studio 服务是否可用，并返回服务版本与支持的 MCP 协议版本。",
            json!({}),
            &[],
            annotations("服务健康", true, false, true),
        ),
        tool(
            "leo_list_projects",
            "剧目列表",
            "分页查询剧目（项目）列表，可按状态、关键字过滤，并按创建/更新时间或名称排序。",
            json!({
                "page": int_prop("页码，从 1 开始，默认 1"),
                "page_size": int_prop("每页数量，1~100，默认 20"),
                "status": str_prop("制作状态过滤，如「制作中」「已完成」"),
                "keyword": str_prop("剧目名称关键字"),
                "sort": enum_prop(&["created_at", "updated_at", "name"], "排序字段，默认 created_at"),
                "order": enum_prop(&["asc", "desc"], "排序方向，默认 desc")
            }),
            &[],
            annotations("剧目列表", true, false, true),
        ),
        tool(
            "leo_get_project",
            "剧目详情",
            "按剧目 ID 查询剧目详情（名称、文件、章节数、总字数、时间）。",
            json!({ "project_id": str_prop("剧目 ID") }),
            &["project_id"],
            annotations("剧目详情", true, false, true),
        ),
        tool(
            "leo_list_chapters",
            "章节大纲",
            "查询剧目下的章节大纲，可按标题关键字过滤，返回章节序号、标题与字数。",
            json!({
                "project_id": str_prop("剧目 ID"),
                "keyword": str_prop("章节标题关键字"),
                "limit": int_prop("最多返回多少章，默认全部")
            }),
            &["project_id"],
            annotations("章节大纲", true, false, true),
        ),
        tool(
            "leo_get_chapter",
            "章节内容",
            "查询单章正文、逐句切分与多音字标注。正文默认按 max_chars 截断，避免上下文过长。",
            json!({
                "project_id": str_prop("剧目 ID"),
                "chapter_id": str_prop("章节 ID"),
                "include_sentences": bool_prop("是否返回逐句切分，默认 true"),
                "max_chars": int_prop("正文字符上限，默认取服务端配置")
            }),
            &["project_id", "chapter_id"],
            annotations("章节内容", true, false, true),
        ),
        tool(
            "leo_search_chapters",
            "搜索章节",
            "在剧目内按关键词搜索章节标题或正文，返回命中章节与摘要片段。",
            json!({
                "project_id": str_prop("剧目 ID"),
                "keyword": str_prop("搜索关键词"),
                "limit": int_prop("最多返回条数，默认 50")
            }),
            &["project_id", "keyword"],
            annotations("搜索章节", true, false, true),
        ),
        tool(
            "leo_list_recycle_bin",
            "回收站列表",
            "分页查询回收站中的剧目（软删除，可恢复）。",
            json!({
                "page": int_prop("页码，从 1 开始，默认 1"),
                "page_size": int_prop("每页数量，1~100，默认 20")
            }),
            &[],
            annotations("回收站列表", true, false, true),
        ),
        tool(
            "leo_get_sentence_polyphones",
            "句子多音字标注",
            "查询某个句子的多音字标注列表（拼音、声调、TTS 拼音）。",
            json!({
                "project_id": str_prop("剧目 ID"),
                "sentence_id": str_prop("句子 ID")
            }),
            &["project_id", "sentence_id"],
            annotations("句子多音字标注", true, false, true),
        ),
        tool(
            "leo_list_chapter_polyphones",
            "章节多音字标注",
            "查询整章的多音字标注，按句子分组返回。",
            json!({
                "project_id": str_prop("剧目 ID"),
                "chapter_id": str_prop("章节 ID")
            }),
            &["project_id", "chapter_id"],
            annotations("章节多音字标注", true, false, true),
        ),
        tool(
            "leo_get_stats",
            "全库统计",
            "返回剧目数、章节数、总字数与回收站数量。",
            json!({}),
            &[],
            annotations("全库统计", true, false, true),
        ),
    ];

    if settings.write_tools {
        tools.extend(write_tools());
    }
    if settings.destructive_tools {
        tools.extend(destructive_tools());
    }

    tools
}

fn write_tools() -> Vec<Tool> {
    vec![
        tool(
            "leo_create_project",
            "新建剧目",
            "用 UTF-8 正文新建剧目，服务端自动清洗与分章。text 与 file_path 二选一。",
            json!({
                "name": str_prop("剧目名称"),
                "text": str_prop("小说正文（UTF-8 纯文本）"),
                "file_path": str_prop("服务器本地 .txt 路径（需开启 file_path 且位于允许目录）")
            }),
            &["name"],
            annotations("新建剧目", false, false, false),
        ),
        tool(
            "leo_append_chapters",
            "追更章节",
            "向已有剧目追加上传章节。dry_run 默认 true，只做差异预检；真正写入需 dry_run=false 且 confirm=true。",
            json!({
                "project_id": str_prop("剧目 ID"),
                "text": str_prop("新章节正文（UTF-8 纯文本）"),
                "file_path": str_prop("服务器本地 .txt 路径（需开启 file_path 且位于允许目录）"),
                "overwrite_existing": bool_prop("是否覆盖同名/同序号章节，默认 false（防脏数据覆盖）"),
                "dry_run": bool_prop("是否只做预检不落盘，默认 true"),
                "confirm": bool_prop("dry_run=false 时必须为 true")
            }),
            &["project_id"],
            annotations("追更章节", false, false, false),
        ),
        tool(
            "leo_rename_project",
            "重命名剧目",
            "修改剧目名称。",
            json!({
                "project_id": str_prop("剧目 ID"),
                "name": str_prop("新名称")
            }),
            &["project_id", "name"],
            annotations("重命名剧目", false, false, true),
        ),
        tool(
            "leo_update_project",
            "更新剧目",
            "更新剧目名称与/或制作状态。",
            json!({
                "project_id": str_prop("剧目 ID"),
                "name": str_prop("新名称"),
                "status": str_prop("制作状态，如「制作中」「已完成」")
            }),
            &["project_id"],
            annotations("更新剧目", false, false, true),
        ),
        tool(
            "leo_save_sentence_polyphones",
            "保存多音字标注",
            "全量覆盖保存某个句子的多音字标注；char_index 为字符下标，必须与句子实际字符一致。",
            json!({
                "project_id": str_prop("剧目 ID"),
                "sentence_id": str_prop("句子 ID"),
                "annotations": {
                    "type": "array",
                    "description": "标注列表",
                    "items": {
                        "type": "object",
                        "properties": {
                            "char_index": int_prop("字符下标（从 0 开始）"),
                            "character": str_prop("该位置的单个汉字"),
                            "pinyin": str_prop("拼音，如 hang"),
                            "pinyin_base": str_prop("基准拼音（可选）"),
                            "tone": int_prop("声调 1~5（可选）"),
                            "tts_pinyin": str_prop("TTS 拼音（可选）")
                        },
                        "required": ["char_index", "character", "pinyin"],
                        "additionalProperties": false
                    }
                }
            }),
            &["project_id", "sentence_id", "annotations"],
            annotations("保存多音字标注", false, false, true),
        ),
        tool(
            "leo_clear_sentence_polyphones",
            "清空多音字标注",
            "删除某个句子的全部多音字标注。",
            json!({
                "project_id": str_prop("剧目 ID"),
                "sentence_id": str_prop("句子 ID")
            }),
            &["project_id", "sentence_id"],
            annotations("清空多音字标注", false, false, true),
        ),
        tool(
            "leo_restore_project",
            "恢复剧目",
            "把回收站中的剧目恢复到剧目库。",
            json!({ "project_id": str_prop("剧目 ID") }),
            &["project_id"],
            annotations("恢复剧目", false, false, true),
        ),
    ]
}

fn destructive_tools() -> Vec<Tool> {
    vec![
        tool(
            "leo_delete_chapter",
            "删除章节",
            "删除章节及其切句与多音字标注，并刷新项目统计。必须 confirm=true。",
            json!({
                "project_id": str_prop("剧目 ID"),
                "chapter_id": str_prop("章节 ID"),
                "confirm": bool_prop("必须为 true 才执行")
            }),
            &["project_id", "chapter_id", "confirm"],
            annotations("删除章节", false, true, true),
        ),
        tool(
            "leo_move_to_trash",
            "移入回收站",
            "软删除剧目，正文与章节保留，可从回收站恢复。必须 confirm=true。",
            json!({
                "project_id": str_prop("剧目 ID"),
                "confirm": bool_prop("必须为 true 才执行")
            }),
            &["project_id", "confirm"],
            annotations("移入回收站", false, true, true),
        ),
        tool(
            "leo_purge_project",
            "彻底删除剧目",
            "物理删除剧目及其文件、章节与切句，不可恢复。必须 confirm=true。",
            json!({
                "project_id": str_prop("剧目 ID"),
                "confirm": bool_prop("必须为 true 才执行")
            }),
            &["project_id", "confirm"],
            annotations("彻底删除剧目", false, true, true),
        ),
        tool(
            "leo_empty_recycle_bin",
            "清空回收站",
            "物理删除回收站中的所有剧目，不可恢复。必须 confirm=true。",
            json!({ "confirm": bool_prop("必须为 true 才执行") }),
            &["confirm"],
            annotations("清空回收站", false, true, true),
        ),
    ]
}

fn value_of<T: serde::Serialize>(value: &T) -> Result<Value, AppError> {
    serde_json::to_value(value).map_err(|err| AppError::internal(format!("序列化响应失败：{err}")))
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
    /// 当前配置下的工具定义。
    pub fn definitions(&self) -> Vec<Tool> {
        (*self.tools).clone()
    }

    pub fn has_tool(&self, name: &str) -> bool {
        self.tools.iter().any(|tool| tool.name.as_ref() == name)
    }

    async fn read_text_input(&self, args: &JsonObject) -> Result<(String, String), AppError> {
        if let Some(text) = opt_string(args, "text") {
            return Ok((text, "model.txt".to_string()));
        }

        let raw_path = req_string(args, "file_path")?;
        let path = self.resolve_input_path(&raw_path)?;
        let filename = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("model.txt")
            .to_string();
        let bytes = std::fs::read(&path)
            .map_err(|err| crate::error::storage_io("读取上传文件失败", err))?;
        if bytes.len() > self.settings.max_bytes() {
            return Err(AppError::file_too_large(format!(
                "文件大小超出限制（最大 {} MB）",
                self.settings.max_bytes() / (1024 * 1024)
            )));
        }
        let text = crate::modules::text_files::service::decode_text(&bytes);
        Ok((text, filename))
    }

    fn resolve_input_path(&self, raw: &str) -> Result<PathBuf, AppError> {
        if !self.mcp.allow_file_path {
            return Err(AppError::invalid_parameter(
                "当前服务未开启 file_path，请改用 text 传入正文",
            ));
        }

        let absolute = crate::config::resolve_path(Path::new(raw));
        let canonical = absolute
            .canonicalize()
            .map_err(|_| AppError::file_not_found(format!("文件不存在：{raw}")))?;
        if !canonical.is_file() {
            return Err(AppError::invalid_parameter(format!("不是文件：{raw}")));
        }

        if !self.mcp.allowed_roots.is_empty() {
            let allowed = self.mcp.allowed_roots.iter().any(|root| {
                root.canonicalize()
                    .map(|base| canonical.starts_with(base))
                    .unwrap_or(false)
            });
            if !allowed {
                return Err(AppError::invalid_parameter(
                    "文件路径不在 LEO_MCP_ALLOWED_ROOTS 允许的目录内",
                ));
            }
        }

        Ok(canonical)
    }

    /// 执行工具调用；错误由调用方包装成工具执行错误。
    pub async fn dispatch(&self, name: &str, args: &JsonObject) -> Result<Value, AppError> {
        match name {
            "leo_health" => Ok(json!({
                "status": "ok",
                "server": "leo-studio",
                "app": self.settings.app_name,
                "version": self.settings.app_version,
                "protocol_versions": ["2026-07-28"],
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
                if let Some(limit) = opt_string(args, "limit").and_then(|v| v.parse::<usize>().ok())
                {
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
                let sentence_id = req_string(args, "sentence_id")?;
                let items = project_service::get_sentence_polyphones(
                    &*self.store.lock().await,
                    &project_id,
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
                let (text, filename) = self.read_text_input(args).await?;
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
                let (text, filename) = self.read_text_input(args).await?;
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
            "leo_rename_project" => {
                let project_id = req_string(args, "project_id")?;
                let name = req_string(args, "name")?;
                let info = project_service::update_project(
                    &mut *self.store.lock().await,
                    &project_id,
                    Some(&name),
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
            "leo_save_sentence_polyphones" => {
                let project_id = req_string(args, "project_id")?;
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

                let items = project_service::save_sentence_polyphones(
                    &mut *self.store.lock().await,
                    &project_id,
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
                let project_id = req_string(args, "project_id")?;
                let sentence_id = req_string(args, "sentence_id")?;
                let deleted_count = project_service::delete_sentence_polyphones(
                    &mut *self.store.lock().await,
                    &project_id,
                    &sentence_id,
                )?;
                Ok(json!({
                    "sentence_id": sentence_id,
                    "deleted_count": deleted_count,
                    "deleted": true
                }))
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
            _ => Err(AppError::invalid_parameter(format!("未知工具：{name}"))),
        }
    }
}
