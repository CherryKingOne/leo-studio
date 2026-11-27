//! 把 canonical 的流式 / 思考配置映射到三套协议的请求字段。
//!
//! 这里只写各协议**标准字段**；供应商专有字段（DeepSeek/方舟的 `thinking`、
//! 百炼的 `enable_thinking` 等）由各供应商插件在协议编码之后再补。

use leo_core::spi::{ReasoningEffort, ThinkingConfig, ThinkingMode};
use serde_json::{Map, Value, json};

/// OpenAI Chat Completions
pub fn apply_chat(body: &mut Map<String, Value>, request: &ThinkingConfig, stream: bool) {
    if stream {
        body.insert("stream".into(), json!(true));
        body.insert("stream_options".into(), json!({ "include_usage": true }));
    }
    if let Some(effort) = effective_effort(request) {
        body.insert("reasoning_effort".into(), json!(effort.as_str()));
    }
    // 单独的“关闭思考”也映射成 none；供应商若用 thinking.type，会在插件层覆盖/补充。
    if request.mode == ThinkingMode::Disabled && request.effort.is_none() {
        body.insert("reasoning_effort".into(), json!("none"));
    }
}

/// OpenAI Responses API
pub fn apply_responses(body: &mut Map<String, Value>, request: &ThinkingConfig, stream: bool) {
    if stream {
        body.insert("stream".into(), json!(true));
    }
    let effort = effective_effort(request);
    if request.mode == ThinkingMode::Disabled {
        body.insert("reasoning".into(), json!({ "effort": "none" }));
    } else if let Some(effort) = effort {
        body.insert("reasoning".into(), json!({ "effort": effort.as_str() }));
    }
}

/// Anthropic Messages
pub fn apply_anthropic(
    body: &mut Map<String, Value>,
    request: &ThinkingConfig,
    stream: bool,
    max_tokens: u32,
) {
    if stream {
        body.insert("stream".into(), json!(true));
    }

    let thinking = match request.mode {
        ThinkingMode::Enabled if max_tokens > 1024 => Some(json!({
            "type": "enabled",
            "budget_tokens": thinking_budget(request.budget_tokens, max_tokens),
        })),
        ThinkingMode::Disabled => Some(json!({ "type": "disabled" })),
        ThinkingMode::Adaptive => Some(json!({ "type": "adaptive" })),
        ThinkingMode::Default if request.budget_tokens.is_some() && max_tokens > 1024 => {
            Some(json!({
                "type": "enabled",
                "budget_tokens": thinking_budget(request.budget_tokens, max_tokens),
            }))
        }
        // max_tokens 太小，Anthropic 无法满足 budget_tokens < max_tokens，索性不传。
        ThinkingMode::Enabled | ThinkingMode::Default => None,
    };
    if let Some(thinking) = thinking {
        body.insert("thinking".into(), thinking);
    }

    let effort = effective_effort(request);
    if let Some(effort) = effort
        && request.mode != ThinkingMode::Disabled
    {
        // Anthropic 的 effort 只支持 low / medium / high / xhigh / max。
        let effort = match effort {
            ReasoningEffort::None | ReasoningEffort::Minimal => "low",
            ReasoningEffort::Low => "low",
            ReasoningEffort::Medium => "medium",
            ReasoningEffort::High => "high",
            ReasoningEffort::XHigh => "xhigh",
            ReasoningEffort::Max => "max",
        };
        body.insert("output_config".into(), json!({ "effort": effort }));
    }
}

fn effective_effort(request: &ThinkingConfig) -> Option<ReasoningEffort> {
    if request.mode == ThinkingMode::Disabled {
        None
    } else {
        request.effort
    }
}

fn thinking_budget(budget: Option<u32>, max_tokens: u32) -> u32 {
    let budget = budget.unwrap_or(4096);
    // Anthropic 要求 budget_tokens < max_tokens。
    if max_tokens > 1024 {
        budget.min(max_tokens - 1).max(1024)
    } else {
        budget.max(1)
    }
}

/// 解析各协议里常见的推理强度字符串。
pub fn parse_effort(value: &str) -> Option<ReasoningEffort> {
    match value.trim().to_ascii_lowercase().as_str() {
        "none" => Some(ReasoningEffort::None),
        "minimal" => Some(ReasoningEffort::Minimal),
        "low" => Some(ReasoningEffort::Low),
        "medium" => Some(ReasoningEffort::Medium),
        "high" => Some(ReasoningEffort::High),
        "xhigh" => Some(ReasoningEffort::XHigh),
        "max" => Some(ReasoningEffort::Max),
        _ => None,
    }
}

fn mode_from_str(value: &str) -> ThinkingMode {
    match value.trim().to_ascii_lowercase().as_str() {
        "enabled" => ThinkingMode::Enabled,
        "disabled" => ThinkingMode::Disabled,
        "adaptive" => ThinkingMode::Adaptive,
        "auto" => ThinkingMode::Adaptive,
        _ => ThinkingMode::Default,
    }
}

/// 从 OpenAI Chat 请求体还原 canonical 思考配置。
pub fn decode_chat(body: &Value) -> ThinkingConfig {
    let mut config = ThinkingConfig::default();
    if let Some(kind) = body
        .get("thinking")
        .and_then(|thinking| thinking.get("type"))
        .and_then(Value::as_str)
    {
        config.mode = mode_from_str(kind);
    }
    if let Some(effort) = body.get("reasoning_effort").and_then(Value::as_str) {
        config.effort = parse_effort(effort);
        if config.effort == Some(ReasoningEffort::None) {
            config.mode = ThinkingMode::Disabled;
            config.effort = None;
        }
    }
    config
}

/// 从 OpenAI Responses 请求体还原 canonical 思考配置。
pub fn decode_responses(body: &Value) -> ThinkingConfig {
    let mut config = ThinkingConfig::default();
    if let Some(effort) = body
        .get("reasoning")
        .and_then(|reasoning| reasoning.get("effort"))
        .and_then(Value::as_str)
    {
        config.effort = parse_effort(effort);
        if config.effort == Some(ReasoningEffort::None) {
            config.mode = ThinkingMode::Disabled;
            config.effort = None;
        }
    }
    config
}

/// 从 Anthropic 请求体还原 canonical 思考配置。
pub fn decode_anthropic(body: &Value) -> ThinkingConfig {
    let mut config = ThinkingConfig::default();
    if let Some(thinking) = body.get("thinking") {
        if let Some(kind) = thinking.get("type").and_then(Value::as_str) {
            config.mode = mode_from_str(kind);
        }
        config.budget_tokens = thinking
            .get("budget_tokens")
            .and_then(Value::as_u64)
            .map(|v| v as u32);
    }
    if let Some(effort) = body
        .get("output_config")
        .and_then(|output| output.get("effort"))
        .and_then(Value::as_str)
    {
        config.effort = parse_effort(effort);
    }
    config
}
