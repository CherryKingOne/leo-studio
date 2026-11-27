use rmcp::model::Tool;
use serde_json::json;

use super::schema::{
    annotations, dialogue_info_schema, dialogue_list_schema, dialogue_unbind_schema, enum_prop,
    int_prop, str_prop, tool,
};

pub fn read_tools() -> Vec<Tool> {
    vec![
        tool(
            "leo_list_dialogues",
            "台词片段列表",
            "分页查询项目章节中的台词与旁白片段，支持片段类型、绑定状态、说话人和关键字过滤。",
            json!({
                "project_id": str_prop("项目 ID"),
                "chapter_id": str_prop("章节 ID"),
                "page": int_prop("页码"),
                "page_size": int_prop("每页数量"),
                "segment_type": enum_prop(&["narration", "dialogue"], "片段类型"),
                "binding_status": enum_prop(&["all", "bound", "unbound"], "绑定状态"),
                "speaker_id": str_prop("说话人角色 ID"),
                "keyword": str_prop("片段正文关键字")
            }),
            &["project_id", "chapter_id"],
            dialogue_list_schema(),
            annotations("台词片段列表", true, false, true),
        ),
        tool(
            "leo_get_dialogue",
            "台词片段详情",
            "查询单个台词片段及其说话人绑定信息。",
            json!({
                "project_id": str_prop("项目 ID"),
                "chapter_id": str_prop("章节 ID"),
                "dialogue_id": str_prop("台词片段 ID")
            }),
            &["project_id", "chapter_id", "dialogue_id"],
            dialogue_info_schema(),
            annotations("台词片段详情", true, false, true),
        ),
        tool(
            "leo_list_unbound_dialogues",
            "未绑定台词",
            "查询章节中尚未绑定说话人的台词与旁白片段。",
            json!({
                "project_id": str_prop("项目 ID"),
                "chapter_id": str_prop("章节 ID"),
                "segment_type": enum_prop(&["narration", "dialogue"], "片段类型"),
                "page": int_prop("页码"),
                "page_size": int_prop("每页数量")
            }),
            &["project_id", "chapter_id"],
            dialogue_list_schema(),
            annotations("未绑定台词", true, false, true),
        ),
    ]
}

pub fn write_tools() -> Vec<Tool> {
    vec![
        tool(
            "leo_bind_dialogue_speaker",
            "绑定台词说话人",
            "将单个台词或旁白片段绑定到当前项目中的角色。",
            json!({
                "project_id": str_prop("项目 ID"),
                "chapter_id": str_prop("章节 ID"),
                "dialogue_id": str_prop("台词片段 ID"),
                "speaker_id": str_prop("说话人角色 ID"),
                "binding_source": enum_prop(&["manual", "auto", "imported"], "绑定来源"),
                "binding_confidence": {
                    "type": "number",
                    "minimum": 0,
                    "maximum": 1,
                    "description": "自动匹配置信度"
                }
            }),
            &["project_id", "chapter_id", "dialogue_id", "speaker_id"],
            dialogue_info_schema(),
            annotations("绑定台词说话人", false, false, true),
        ),
        tool(
            "leo_unbind_dialogue_speaker",
            "解绑台词说话人",
            "清除单个台词或旁白片段的说话人绑定，操作幂等。",
            json!({
                "project_id": str_prop("项目 ID"),
                "chapter_id": str_prop("章节 ID"),
                "dialogue_id": str_prop("台词片段 ID")
            }),
            &["project_id", "chapter_id", "dialogue_id"],
            dialogue_unbind_schema(),
            annotations("解绑台词说话人", false, false, true),
        ),
    ]
}
