//! 火山方舟（豆包）内置兜底模型清单。
//!
//! 方舟实际使用推理接入点（`ep-...`）或控制台已开通的模型 ID，静态清单只用于展示 /
//! 兜底；配置里的 `model` 不在清单里时插入到第一位。

use leo_core::spi::ModelInfo;

pub fn builtin_models(default_model: &str) -> Vec<ModelInfo> {
    let mut models = vec![
        ModelInfo {
            id: "doubao-seed-1-6".into(),
            name: "豆包 Seed 1.6".into(),
            context_window: Some(256_000),
            capabilities: vec!["chat".into(), "json".into()],
        },
        ModelInfo {
            id: "doubao-seed-1-6-flash".into(),
            name: "豆包 Seed 1.6 Flash".into(),
            context_window: Some(256_000),
            capabilities: vec!["chat".into(), "json".into()],
        },
        ModelInfo {
            id: "doubao-seed-1-6-thinking".into(),
            name: "豆包 Seed 1.6 Thinking".into(),
            context_window: Some(256_000),
            capabilities: vec!["chat".into(), "reasoning".into()],
        },
        ModelInfo {
            id: "doubao-pro-32k".into(),
            name: "豆包 Pro 32K".into(),
            context_window: Some(32_768),
            capabilities: vec!["chat".into()],
        },
        ModelInfo {
            id: "doubao-lite-32k".into(),
            name: "豆包 Lite 32K".into(),
            context_window: Some(32_768),
            capabilities: vec!["chat".into()],
        },
    ];
    ensure_default(&mut models, default_model);
    models
}

fn ensure_default(models: &mut Vec<ModelInfo>, default_model: &str) {
    let default_model = default_model.trim();
    if default_model.is_empty() || models.iter().any(|model| model.id == default_model) {
        return;
    }
    models.insert(
        0,
        ModelInfo {
            id: default_model.to_string(),
            name: default_model.to_string(),
            context_window: None,
            capabilities: vec!["chat".into()],
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoint_is_inserted_first() {
        let models = builtin_models("ep-2026xxxx");
        assert_eq!(models[0].id, "ep-2026xxxx");
    }
}
