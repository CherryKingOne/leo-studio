use rmcp::model::Tool;
use serde_json::json;

use super::schema::{
    annotations, enum_prop, int_prop, output_schema, project_info_schema, project_list_schema,
    str_prop, tool,
};

pub fn read_tools() -> Vec<Tool> {
    vec![
        tool(
            "leo_list_projects",
            "剧目列表",
            "分页查询剧目列表，支持状态、关键字过滤与排序。",
            json!({
                "page": int_prop("页码，从 1 开始，默认 1"),
                "page_size": int_prop("每页数量，1~100，默认 20"),
                "status": str_prop("制作状态过滤"),
                "keyword": str_prop("剧目名称关键字"),
                "sort": enum_prop(&["created_at", "updated_at", "name"], "排序字段"),
                "order": enum_prop(&["asc", "desc"], "排序方向")
            }),
            &[],
            project_list_schema(),
            annotations("剧目列表", true, false, true),
        ),
        tool(
            "leo_get_project",
            "剧目详情",
            "按项目 ID 查询剧目详情。",
            json!({ "project_id": str_prop("剧目 ID") }),
            &["project_id"],
            project_info_schema(),
            annotations("剧目详情", true, false, true),
        ),
        tool(
            "leo_get_stats",
            "全库统计",
            "返回剧目数、章节数、总字数与回收站数量。",
            json!({}),
            &[],
            output_schema(
                json!({
                    "projects": { "type": "integer" },
                    "chapters": { "type": "integer" },
                    "words": { "type": "integer" },
                    "recycle_bin": { "type": "integer" }
                }),
                &["projects", "chapters", "words", "recycle_bin"],
            ),
            annotations("全库统计", true, false, true),
        ),
    ]
}

pub fn write_tools() -> Vec<Tool> {
    vec![
        tool(
            "leo_create_project",
            "新建剧目",
            "用 UTF-8 正文新建剧目，服务端自动清洗与分章。",
            json!({
                "name": str_prop("剧目名称"),
                "text": str_prop("小说正文")
            }),
            &["name", "text"],
            project_info_schema(),
            annotations("新建剧目", false, false, false),
        ),
        tool(
            "leo_update_project",
            "更新剧目",
            "更新剧目名称与/或制作状态。",
            json!({
                "project_id": str_prop("剧目 ID"),
                "name": str_prop("新名称"),
                "status": str_prop("制作状态")
            }),
            &["project_id"],
            project_info_schema(),
            annotations("更新剧目", false, false, true),
        ),
    ]
}
