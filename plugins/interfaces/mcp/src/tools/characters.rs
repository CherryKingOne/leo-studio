use rmcp::model::Tool;
use serde_json::json;

use super::schema::{
    annotations, bool_prop, character_info_schema, character_list_schema, enum_prop, int_prop,
    output_schema, str_prop, tool,
};

pub fn read_tools() -> Vec<Tool> {
    vec![
        tool(
            "leo_list_characters",
            "角色列表",
            "分页查询项目角色，支持关键字、性别、类型、状态和标签过滤。",
            json!({
                "project_id": str_prop("剧目 ID"),
                "page": int_prop("页码"),
                "page_size": int_prop("每页数量"),
                "keyword": str_prop("名称、别名或简介关键字"),
                "gender": enum_prop(&["male", "female", "unknown", "other"], "性别"),
                "role_type": enum_prop(
                    &["protagonist", "supporting", "antagonist", "narrator", "other"],
                    "角色类型"
                ),
                "status": enum_prop(&["active", "archived", "all"], "状态"),
                "tag": str_prop("标签"),
                "sort": enum_prop(&["created_at", "updated_at", "name"], "排序字段"),
                "order": enum_prop(&["asc", "desc"], "排序方向")
            }),
            &["project_id"],
            character_list_schema(),
            annotations("角色列表", true, false, true),
        ),
        tool(
            "leo_get_character",
            "角色详情",
            "按项目 ID 与角色 ID 查询角色档案。",
            json!({
                "project_id": str_prop("剧目 ID"),
                "character_id": str_prop("角色 ID")
            }),
            &["project_id", "character_id"],
            character_info_schema(),
            annotations("角色详情", true, false, true),
        ),
    ]
}

pub fn write_tools() -> Vec<Tool> {
    vec![
        tool(
            "leo_create_character",
            "新建角色",
            "在指定项目中创建角色档案。",
            json!({
                "project_id": str_prop("剧目 ID"),
                "name": str_prop("角色名称"),
                "aliases": {
                    "type": "array",
                    "items": { "type": "string" },
                    "maxItems": 20
                },
                "gender": enum_prop(&["male", "female", "unknown", "other"], "性别"),
                "age": str_prop("年龄描述"),
                "role_type": enum_prop(
                    &["protagonist", "supporting", "antagonist", "narrator", "other"],
                    "角色类型"
                ),
                "status": enum_prop(&["active", "archived"], "角色状态"),
                "description": str_prop("角色简介"),
                "tags": {
                    "type": "array",
                    "items": { "type": "string" },
                    "maxItems": 20
                }
            }),
            &["project_id", "name"],
            character_info_schema(),
            annotations("新建角色", false, false, false),
        ),
        tool(
            "leo_update_character",
            "更新角色",
            "更新角色档案，未提供的字段保持不变。",
            json!({
                "project_id": str_prop("剧目 ID"),
                "character_id": str_prop("角色 ID"),
                "name": str_prop("角色名称"),
                "aliases": {
                    "type": "array",
                    "items": { "type": "string" },
                    "maxItems": 20
                },
                "gender": enum_prop(&["male", "female", "unknown", "other"], "性别"),
                "age": {
                    "type": ["string", "null"],
                    "description": "年龄描述，null 表示清空"
                },
                "role_type": enum_prop(
                    &["protagonist", "supporting", "antagonist", "narrator", "other"],
                    "角色类型"
                ),
                "status": enum_prop(&["active", "archived"], "角色状态"),
                "description": str_prop("角色简介"),
                "tags": {
                    "type": "array",
                    "items": { "type": "string" },
                    "maxItems": 20
                }
            }),
            &["project_id", "character_id"],
            character_info_schema(),
            annotations("更新角色", false, false, true),
        ),
    ]
}

pub fn destructive_tools() -> Vec<Tool> {
    vec![tool(
        "leo_delete_character",
        "删除角色",
        "删除项目中的角色档案，必须 confirm=true。",
        json!({
            "project_id": str_prop("剧目 ID"),
            "character_id": str_prop("角色 ID"),
            "confirm": bool_prop("必须为 true")
        }),
        &["project_id", "character_id", "confirm"],
        output_schema(
            json!({
                "project_id": { "type": "string" },
                "character_id": { "type": "string" },
                "name": { "type": "string" },
                "deleted": { "type": "boolean" }
            }),
            &["project_id", "character_id", "name", "deleted"],
        ),
        annotations("删除角色", false, true, true),
    )]
}
