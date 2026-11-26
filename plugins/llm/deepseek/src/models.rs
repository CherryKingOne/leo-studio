//! DeepSeek 内置兜底模型清单。
//!
//! 远端模型列表接口不保证每家都开放，所以 `list_models` 直接返回这份静态清单；
//! 配置里的 `model` 不在清单里时插入到第一位。

use leo_core::spi::ModelInfo;

pub fn builtin_models(default_model: &str) -> Vec<ModelInfo> {
    let mut models = vec![
        ModelInfo {
            id: "deepseek-chat".into(),
            name: "DeepSeek Chat".into(),
            context_window: Some(65_536),
            capabilities: vec!["chat".into(), "json".into()],
        },
        ModelInfo {
            id: "deepseek-reasoner".into(),
            name: "DeepSeek Reasoner".into(),
            context_window: Some(65_536),
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
        let models = builtin_models("my-endpoint");
        assert_eq!(models[0].id, "my-endpoint");
        assert_eq!(models[1].id, "deepseek-chat");
    }
}
