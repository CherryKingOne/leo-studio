use std::sync::Arc;

use rmcp::model::{JsonObject, Tool, ToolAnnotations};
use serde_json::{Map, Value, json};

pub fn str_prop(description: &str) -> Value {
    json!({ "type": "string", "description": description })
}

pub fn int_prop(description: &str) -> Value {
    json!({ "type": "integer", "description": description })
}

pub fn bool_prop(description: &str) -> Value {
    json!({ "type": "boolean", "description": description })
}

pub fn enum_prop(values: &[&str], description: &str) -> Value {
    json!({ "type": "string", "enum": values, "description": description })
}

pub fn object_schema(properties: Value, required: &[&str]) -> Arc<JsonObject> {
    let mut map = Map::new();
    map.insert("type".to_string(), json!("object"));
    map.insert("properties".to_string(), properties);
    map.insert("required".to_string(), json!(required));
    map.insert("additionalProperties".to_string(), json!(false));
    Arc::new(map)
}

pub fn output_schema(properties: Value, required: &[&str]) -> Arc<JsonObject> {
    let mut map = Map::new();
    map.insert("type".to_string(), json!("object"));
    map.insert("properties".to_string(), properties);
    map.insert("required".to_string(), json!(required));
    map.insert("additionalProperties".to_string(), json!(false));
    Arc::new(map)
}

pub fn annotations(
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
pub fn tool(
    name: &'static str,
    title: &'static str,
    description: &'static str,
    properties: Value,
    required: &[&str],
    output: Arc<JsonObject>,
    tool_annotations: ToolAnnotations,
) -> Tool {
    let mut item = Tool::default();
    item.name = name.into();
    item.title = Some(title.to_string());
    item.description = Some(description.into());
    item.input_schema = object_schema(properties, required);
    item.output_schema = Some(output);
    item.annotations = Some(tool_annotations);
    item
}

fn string(description: &str) -> Value {
    json!({ "type": "string", "description": description })
}

fn nullable_string(description: &str) -> Value {
    json!({ "type": ["string", "null"], "description": description })
}

fn integer(description: &str) -> Value {
    json!({ "type": "integer", "description": description })
}

fn boolean(description: &str) -> Value {
    json!({ "type": "boolean", "description": description })
}

fn string_array(description: &str) -> Value {
    json!({
        "type": "array",
        "description": description,
        "items": { "type": "string" }
    })
}

fn array(items: impl serde::Serialize, description: &str) -> Value {
    json!({ "type": "array", "description": description, "items": items })
}

pub fn health_schema() -> Arc<JsonObject> {
    output_schema(
        json!({
            "status": string("健康状态"),
            "server": string("服务名称"),
            "app": string("应用名称"),
            "version": string("应用版本"),
            "protocol_versions": string_array("支持的 MCP 协议版本"),
            "write_tools": boolean("是否启用写工具"),
            "destructive_tools": boolean("是否启用破坏性工具")
        }),
        &[
            "status",
            "server",
            "app",
            "version",
            "protocol_versions",
            "write_tools",
            "destructive_tools",
        ],
    )
}

pub fn project_info_schema() -> Arc<JsonObject> {
    output_schema(
        json!({
            "project_id": string("项目 ID"),
            "name": string("项目名称"),
            "file_id": string("文件 ID"),
            "filename": string("文件名"),
            "status": string("制作状态"),
            "chapter_count": integer("章节数"),
            "total_words": integer("总字数"),
            "created_at": string("创建时间"),
            "updated_at": string("更新时间"),
            "is_deleted": boolean("是否在回收站"),
            "deleted_at": nullable_string("删除时间")
        }),
        &[
            "project_id",
            "name",
            "file_id",
            "filename",
            "status",
            "chapter_count",
            "total_words",
            "created_at",
            "updated_at",
            "is_deleted",
            "deleted_at",
        ],
    )
}

pub fn project_list_schema() -> Arc<JsonObject> {
    output_schema(
        json!({
            "total": integer("总记录数"),
            "page": integer("当前页"),
            "page_size": integer("每页数量"),
            "pages": integer("总页数"),
            "items": array(project_info_schema(), "项目列表")
        }),
        &["total", "page", "page_size", "pages", "items"],
    )
}

pub fn chapter_outline_schema() -> Arc<JsonObject> {
    output_schema(
        json!({
            "chapter_id": string("章节 ID"),
            "chapter_index": integer("章节序号"),
            "chapter_title": string("章节标题"),
            "word_count": integer("章节字数")
        }),
        &["chapter_id", "chapter_index", "chapter_title", "word_count"],
    )
}

pub fn chapter_outline_list_schema() -> Arc<JsonObject> {
    output_schema(
        json!({
            "project_id": string("项目 ID"),
            "total": integer("章节数"),
            "items": array(chapter_outline_schema(), "章节大纲")
        }),
        &["project_id", "total", "items"],
    )
}

pub fn sentence_polyphone_schema() -> Arc<JsonObject> {
    output_schema(
        json!({
            "polyphone_id": string("标注 ID"),
            "sentence_id": string("句子 ID"),
            "char_index": integer("字符下标"),
            "character": string("被标注字符"),
            "pinyin": string("原始拼音"),
            "pinyin_base": string("基础拼音"),
            "tone": integer("声调"),
            "tts_pinyin": string("TTS 拼音"),
            "created_at": string("创建时间"),
            "updated_at": string("更新时间")
        }),
        &[
            "polyphone_id",
            "sentence_id",
            "char_index",
            "character",
            "pinyin",
            "pinyin_base",
            "tone",
            "tts_pinyin",
            "created_at",
            "updated_at",
        ],
    )
}

pub fn chapter_content_schema() -> Arc<JsonObject> {
    let sentence = output_schema(
        json!({
            "sentence_id": string("句子 ID"),
            "sentence_index": integer("句子序号"),
            "content": string("句子内容"),
            "word_count": integer("句子字数"),
            "polyphones": array(sentence_polyphone_schema(), "多音字标注")
        }),
        &[
            "sentence_id",
            "sentence_index",
            "content",
            "word_count",
            "polyphones",
        ],
    );
    output_schema(
        json!({
            "project_id": string("项目 ID"),
            "file_id": string("文件 ID"),
            "chapter_id": string("章节 ID"),
            "chapter_index": integer("章节序号"),
            "chapter_title": string("章节标题"),
            "word_count": integer("章节字数"),
            "content": string("章节正文"),
            "sentences": array(sentence, "切句列表"),
            "truncated": boolean("是否发生截断")
        }),
        &[
            "project_id",
            "file_id",
            "chapter_id",
            "chapter_index",
            "chapter_title",
            "word_count",
            "content",
            "sentences",
            "truncated",
        ],
    )
}

pub fn character_info_schema() -> Arc<JsonObject> {
    output_schema(
        json!({
            "character_id": string("角色 ID"),
            "project_id": string("项目 ID"),
            "name": string("角色名称"),
            "aliases": string_array("别名"),
            "gender": string("性别"),
            "age": nullable_string("年龄"),
            "role_type": string("角色类型"),
            "status": string("角色状态"),
            "description": string("角色简介"),
            "tags": string_array("标签"),
            "created_at": string("创建时间"),
            "updated_at": string("更新时间")
        }),
        &[
            "character_id",
            "project_id",
            "name",
            "aliases",
            "gender",
            "age",
            "role_type",
            "status",
            "description",
            "tags",
            "created_at",
            "updated_at",
        ],
    )
}

pub fn character_list_schema() -> Arc<JsonObject> {
    output_schema(
        json!({
            "project_id": string("项目 ID"),
            "total": integer("总记录数"),
            "page": integer("当前页"),
            "page_size": integer("每页数量"),
            "pages": integer("总页数"),
            "items": array(character_info_schema(), "角色列表")
        }),
        &["project_id", "total", "page", "page_size", "pages", "items"],
    )
}

pub fn sentence_polyphone_list_schema() -> Arc<JsonObject> {
    output_schema(
        json!({
            "sentence_id": string("句子 ID"),
            "total": integer("标注总数"),
            "items": array(sentence_polyphone_schema(), "标注列表")
        }),
        &["sentence_id", "total", "items"],
    )
}

pub fn chapter_polyphone_list_schema() -> Arc<JsonObject> {
    let sentence = output_schema(
        json!({
            "sentence_id": string("句子 ID"),
            "sentence_index": integer("句子序号"),
            "content": string("句子内容"),
            "polyphones": array(sentence_polyphone_schema(), "多音字标注")
        }),
        &["sentence_id", "sentence_index", "content", "polyphones"],
    );
    output_schema(
        json!({
            "chapter_id": string("章节 ID"),
            "chapter_title": string("章节标题"),
            "sentence_count": integer("含标注的句子数"),
            "total": integer("标注总数"),
            "items": array(sentence, "句子标注列表")
        }),
        &[
            "chapter_id",
            "chapter_title",
            "sentence_count",
            "total",
            "items",
        ],
    )
}

pub fn delete_result_schema(fields: Value) -> Arc<JsonObject> {
    output_schema(fields, &[])
}
