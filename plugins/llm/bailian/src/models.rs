//! 阿里云百炼（Qwen）内置兜底模型清单。
//!
//! 远端模型列表接口不保证可用，所以 `list_models` 直接返回这份静态清单；
//! 配置里的 `model` 不在清单里时插入到第一位。

use leo_core::spi::ModelInfo;

pub fn builtin_models(default_model: &str) -> Vec<ModelInfo> {
    let mut models = vec![
        ModelInfo {
            id: "qwen-max".into(),
            name: "Qwen Max".into(),
            context_window: Some(131_072),
            capabilities: vec!["chat".into(), "json".into()],
        },
        ModelInfo {
            id: "qwen-plus".into(),
            name: "Qwen Plus".into(),
            context_window: Some(131_072),
            capabilities: vec!["chat".into(), "json".into()],
        },
        ModelInfo {
            id: "qwen-turbo".into(),
            name: "Qwen Turbo".into(),
            context_window: Some(131_072),
            capabilities: vec!["chat".into(), "json".into()],
        },
        ModelInfo {
            id: "qwen-long".into(),
            name: "Qwen Long".into(),
            context_window: Some(1_000_000),
            capabilities: vec!["chat".into(), "long_context".into()],
        },
        ModelInfo {
            id: "qwen3-235b-a22b".into(),
            name: "Qwen3 235B".into(),
            context_window: Some(131_072),
            capabilities: vec!["chat".into(), "reasoning".into()],
        },
        ModelInfo {
            id: "qwen3-32b".into(),
            name: "Qwen3 32B".into(),
            context_window: Some(131_072),
            capabilities: vec!["chat".into(), "reasoning".into()],
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
    fn custom_default_is_inserted() {
        let models = builtin_models("qwen3-coder-plus");
        assert_eq!(models[0].id, "qwen3-coder-plus");
        assert_eq!(models[1].id, "qwen-max");
    }
}
