use rmcp::model::Tool;
use serde_json::json;

use super::schema::{annotations, health_schema, tool};

pub fn read_tools() -> Vec<Tool> {
    vec![tool(
        "leo_health",
        "服务健康",
        "检查 Leo Studio MCP 服务是否可用，并返回版本与工具开关状态。",
        json!({}),
        &[],
        health_schema(),
        annotations("服务健康", true, false, true),
    )]
}
