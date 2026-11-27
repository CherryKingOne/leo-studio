use rmcp::model::Tool;
use serde_json::json;

use super::schema::{
    annotations, bool_prop, chapter_polyphone_list_schema, int_prop, output_schema,
    sentence_polyphone_list_schema, str_prop, tool,
};

pub fn read_tools() -> Vec<Tool> {
    vec![
        tool(
            "leo_list_chapter_polyphones",
            "章节多音字标注",
            "查询整章的多音字标注，按句子分组返回。",
            json!({
                "project_id": str_prop("剧目 ID"),
                "chapter_id": str_prop("章节 ID")
            }),
            &["project_id", "chapter_id"],
            chapter_polyphone_list_schema(),
            annotations("章节多音字标注", true, false, true),
        ),
        tool(
            "leo_get_sentence_polyphones",
            "句子多音字标注",
            "查询项目章节下某个句子的多音字标注。",
            json!({
                "project_id": str_prop("剧目 ID"),
                "chapter_id": str_prop("章节 ID"),
                "sentence_id": str_prop("句子 ID")
            }),
            &["project_id", "chapter_id", "sentence_id"],
            sentence_polyphone_list_schema(),
            annotations("句子多音字标注", true, false, true),
        ),
    ]
}

pub fn write_tools() -> Vec<Tool> {
    vec![tool(
        "leo_save_sentence_polyphones",
        "保存多音字标注",
        "全量覆盖保存项目章节下某个句子的多音字标注。",
        json!({
            "project_id": str_prop("剧目 ID"),
            "chapter_id": str_prop("章节 ID"),
            "sentence_id": str_prop("句子 ID"),
            "annotations": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "char_index": int_prop("字符下标"),
                        "character": str_prop("单个字符"),
                        "pinyin": str_prop("拼音"),
                        "pinyin_base": str_prop("基础拼音"),
                        "tone": int_prop("声调"),
                        "tts_pinyin": str_prop("TTS 拼音")
                    },
                    "required": ["char_index", "character", "pinyin"],
                    "additionalProperties": false
                }
            }
        }),
        &["project_id", "chapter_id", "sentence_id", "annotations"],
        sentence_polyphone_list_schema(),
        annotations("保存多音字标注", false, false, true),
    )]
}

pub fn destructive_tools() -> Vec<Tool> {
    vec![
        tool(
            "leo_clear_sentence_polyphones",
            "清空多音字标注",
            "清空项目章节下某个句子的全部多音字标注，必须 confirm=true。",
            json!({
                "project_id": str_prop("剧目 ID"),
                "chapter_id": str_prop("章节 ID"),
                "sentence_id": str_prop("句子 ID"),
                "confirm": bool_prop("必须为 true")
            }),
            &["project_id", "chapter_id", "sentence_id", "confirm"],
            output_schema(
                json!({
                    "sentence_id": { "type": "string" },
                    "deleted_count": { "type": "integer" },
                    "deleted": { "type": "boolean" }
                }),
                &["sentence_id", "deleted_count", "deleted"],
            ),
            annotations("清空多音字标注", false, true, true),
        ),
        tool(
            "leo_delete_sentence_polyphone",
            "删除单条多音字标注",
            "删除句子中指定字符下标的多音字标注，必须 confirm=true。",
            json!({
                "project_id": str_prop("剧目 ID"),
                "chapter_id": str_prop("章节 ID"),
                "sentence_id": str_prop("句子 ID"),
                "char_index": int_prop("字符下标"),
                "confirm": bool_prop("必须为 true")
            }),
            &[
                "project_id",
                "chapter_id",
                "sentence_id",
                "char_index",
                "confirm",
            ],
            output_schema(
                json!({
                    "polyphone_id": { "type": "string" },
                    "sentence_id": { "type": "string" },
                    "char_index": { "type": "integer" },
                    "deleted": { "type": "boolean" }
                }),
                &["polyphone_id", "sentence_id", "char_index", "deleted"],
            ),
            annotations("删除单条多音字标注", false, true, true),
        ),
    ]
}
