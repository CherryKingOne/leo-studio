use rmcp::model::Tool;
use serde_json::json;

use super::schema::{
    annotations, bool_prop, chapter_content_schema, chapter_outline_list_schema, int_prop,
    output_schema, str_prop, tool,
};

pub fn read_tools() -> Vec<Tool> {
    vec![
        tool(
            "leo_list_chapters",
            "章节大纲",
            "查询剧目下的章节大纲，可按标题关键字过滤。",
            json!({
                "project_id": str_prop("剧目 ID"),
                "keyword": str_prop("章节标题关键字"),
                "limit": int_prop("最多返回多少章")
            }),
            &["project_id"],
            chapter_outline_list_schema(),
            annotations("章节大纲", true, false, true),
        ),
        tool(
            "leo_get_chapter",
            "章节内容",
            "查询单章正文、逐句切分与多音字标注。",
            json!({
                "project_id": str_prop("剧目 ID"),
                "chapter_id": str_prop("章节 ID"),
                "include_sentences": bool_prop("是否返回逐句切分"),
                "max_chars": int_prop("正文字符上限")
            }),
            &["project_id", "chapter_id"],
            chapter_content_schema(),
            annotations("章节内容", true, false, true),
        ),
        tool(
            "leo_search_chapters",
            "搜索章节",
            "在剧目内搜索章节标题或正文。",
            json!({
                "project_id": str_prop("剧目 ID"),
                "keyword": str_prop("搜索关键词"),
                "limit": int_prop("最多返回条数")
            }),
            &["project_id", "keyword"],
            output_schema(
                json!({
                    "project_id": { "type": "string" },
                    "total": { "type": "integer" },
                    "items": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "chapter_id": { "type": "string" },
                                "chapter_index": { "type": "integer" },
                                "chapter_title": { "type": "string" },
                                "word_count": { "type": "integer" },
                                "excerpt": { "type": "string" }
                            },
                            "required": ["chapter_id", "chapter_index", "chapter_title", "word_count", "excerpt"],
                            "additionalProperties": false
                        }
                    }
                }),
                &["project_id", "total", "items"],
            ),
            annotations("搜索章节", true, false, true),
        ),
    ]
}

pub fn write_tools() -> Vec<Tool> {
    vec![
        tool(
            "leo_append_chapters",
            "追更章节",
            "追加章节正文。dry_run 默认 true，实际写入必须 confirm=true。",
            json!({
                "project_id": str_prop("剧目 ID"),
                "text": str_prop("新章节正文"),
                "overwrite_existing": bool_prop("是否覆盖同名章节"),
                "dry_run": bool_prop("是否只做预检"),
                "confirm": bool_prop("实际写入时必须为 true")
            }),
            &["project_id", "text"],
            output_schema(
                json!({
                    "project_id": { "type": "string" },
                    "file_id": { "type": "string" },
                    "dry_run": { "type": "boolean" },
                    "added_chapters": { "type": "integer" },
                    "skipped_chapters": { "type": "integer" },
                    "total_chapters": { "type": "integer" },
                    "total_words": { "type": "integer" },
                    "has_conflicts": { "type": "boolean" },
                    "conflicts": { "type": "array", "items": { "type": "object" } },
                    "chapters": { "type": "array", "items": { "type": "object" } },
                    "updated_at": { "type": "string" }
                }),
                &[
                    "project_id",
                    "file_id",
                    "dry_run",
                    "added_chapters",
                    "skipped_chapters",
                    "total_chapters",
                    "total_words",
                    "has_conflicts",
                    "conflicts",
                    "chapters",
                    "updated_at",
                ],
            ),
            annotations("追更章节", false, false, false),
        ),
        tool(
            "leo_update_chapter",
            "更新章节",
            "修改章节标题与/或正文。正文变更必须 confirm=true。",
            json!({
                "project_id": str_prop("剧目 ID"),
                "chapter_id": str_prop("章节 ID"),
                "title": str_prop("新章节标题"),
                "content": str_prop("新正文"),
                "confirm": bool_prop("修改正文时必须为 true")
            }),
            &["project_id", "chapter_id"],
            chapter_content_schema(),
            annotations("更新章节", false, false, false),
        ),
    ]
}

pub fn destructive_tools() -> Vec<Tool> {
    vec![tool(
        "leo_delete_chapter",
        "删除章节",
        "删除章节及其切句与多音字标注，必须 confirm=true。",
        json!({
            "project_id": str_prop("剧目 ID"),
            "chapter_id": str_prop("章节 ID"),
            "confirm": bool_prop("必须为 true")
        }),
        &["project_id", "chapter_id", "confirm"],
        output_schema(
            json!({
                "project_id": { "type": "string" },
                "chapter_id": { "type": "string" },
                "chapter_title": { "type": "string" },
                "deleted": { "type": "boolean" },
                "chapter_count": { "type": "integer" },
                "total_words": { "type": "integer" },
                "updated_at": { "type": "string" }
            }),
            &[
                "project_id",
                "chapter_id",
                "chapter_title",
                "deleted",
                "chapter_count",
                "total_words",
                "updated_at",
            ],
        ),
        annotations("删除章节", false, true, true),
    )]
}
