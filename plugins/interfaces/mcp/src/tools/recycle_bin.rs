use rmcp::model::Tool;
use serde_json::json;

use super::schema::{
    annotations, bool_prop, int_prop, output_schema, project_list_schema, str_prop, tool,
};

pub fn read_tools() -> Vec<Tool> {
    vec![tool(
        "leo_list_recycle_bin",
        "回收站列表",
        "分页查询回收站中的剧目。",
        json!({
            "page": int_prop("页码"),
            "page_size": int_prop("每页数量")
        }),
        &[],
        project_list_schema(),
        annotations("回收站列表", true, false, true),
    )]
}

pub fn write_tools() -> Vec<Tool> {
    vec![tool(
        "leo_restore_project",
        "恢复剧目",
        "把回收站中的剧目恢复到剧目库。",
        json!({ "project_id": str_prop("剧目 ID") }),
        &["project_id"],
        output_schema(
            json!({
                "project_id": { "type": "string" },
                "name": { "type": "string" },
                "restored": { "type": "boolean" }
            }),
            &["project_id", "name", "restored"],
        ),
        annotations("恢复剧目", false, false, true),
    )]
}

pub fn destructive_tools() -> Vec<Tool> {
    vec![
        tool(
            "leo_move_to_trash",
            "移入回收站",
            "软删除剧目，必须 confirm=true。",
            json!({
                "project_id": str_prop("剧目 ID"),
                "confirm": bool_prop("必须为 true")
            }),
            &["project_id", "confirm"],
            output_schema(
                json!({
                    "project_id": { "type": "string" },
                    "name": { "type": "string" },
                    "deleted": { "type": "boolean" },
                    "deleted_at": { "type": ["string", "null"] }
                }),
                &["project_id", "name", "deleted", "deleted_at"],
            ),
            annotations("移入回收站", false, true, true),
        ),
        tool(
            "leo_purge_project",
            "彻底删除剧目",
            "物理删除剧目及其关联数据，必须 confirm=true。",
            json!({
                "project_id": str_prop("剧目 ID"),
                "confirm": bool_prop("必须为 true")
            }),
            &["project_id", "confirm"],
            output_schema(
                json!({
                    "project_id": { "type": "string" },
                    "name": { "type": "string" },
                    "hard_deleted": { "type": "boolean" }
                }),
                &["project_id", "name", "hard_deleted"],
            ),
            annotations("彻底删除剧目", false, true, true),
        ),
        tool(
            "leo_empty_recycle_bin",
            "清空回收站",
            "物理删除回收站中的所有剧目，必须 confirm=true。",
            json!({ "confirm": bool_prop("必须为 true") }),
            &["confirm"],
            output_schema(
                json!({
                    "cleared_count": { "type": "integer" },
                    "cleared": { "type": "boolean" }
                }),
                &["cleared_count", "cleared"],
            ),
            annotations("清空回收站", false, true, true),
        ),
    ]
}
