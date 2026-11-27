//! 百度千帆 自有模型清单与边界。
//!
//! 只接受 百度千帆 自家模型；即使上游模型列表里混入其他厂商的模型，也会被过滤。
//! 模型能力 / 上下文来自官方模型页，厂商上新后改这里即可；也支持用
//! `allowed_models` 补充私有化模型。

use leo_core::spi::ModelInfo;

/// 自家模型前缀（大小写不敏感）
pub const OWNED_PREFIXES: &[&str] = &["ernie-"];

/// 明显不是对话模型的标记，避免把 embedding / 语音 / 图像类模型当聊天模型展示
pub const DENIED_MARKERS: &[&str] = &[
    "embedding",
    "rerank",
    "tts",
    "speech",
    "audio",
    "voice",
    "realtime",
    "image",
    "video",
    "music",
    "live",
    "omni",
    "genie",
    "lyria",
    "nano-banana",
    "transcribe",
    "hailuo",
];

pub fn is_owned_model(model: &str, allowed: &[String]) -> bool {
    let id = model.trim();
    if id.is_empty() || id.contains(':') {
        return false;
    }
    let lower = id.to_ascii_lowercase();
    if DENIED_MARKERS.iter().any(|marker| lower.contains(marker)) {
        return false;
    }
    if allowed.iter().any(|item| item.eq_ignore_ascii_case(id)) {
        return true;
    }
    OWNED_PREFIXES
        .iter()
        .any(|prefix| lower.starts_with(prefix))
}

fn model(id: &str, name: &str, context_window: u32, capabilities: &[&str]) -> ModelInfo {
    ModelInfo {
        id: id.into(),
        name: name.into(),
        context_window: Some(context_window),
        capabilities: capabilities.iter().map(|item| (*item).into()).collect(),
    }
}

/// 内置模型目录。默认模型放最前面；远端列表失败时用它兜底。
pub fn builtin_models(default_model: &str) -> Vec<ModelInfo> {
    let mut models = vec![
        model(
            "ernie-5.5-flash-preview",
            "ERNIE-5.5-Flash-Preview",
            1_048_576,
            &["chat", "reasoning"],
        ),
        model("ernie-5.1", "ERNIE-5.1", 131_072, &["chat"]),
        model(
            "ernie-5.0",
            "ERNIE-5.0",
            131_072,
            &["chat", "vision", "reasoning"],
        ),
        model(
            "ernie-4.5-turbo-128k",
            "ERNIE-4.5-Turbo-128K",
            131_072,
            &["chat"],
        ),
        model(
            "ernie-4.5-vl-28b-a3b",
            "ERNIE-4.5-VL-28B-A3B",
            131_072,
            &["chat", "vision"],
        ),
    ];
    ensure_default(&mut models, default_model);
    models
}

/// 保证 `default_model` 一定出现在列表里（支持用户自定义模型）。
pub fn ensure_default(models: &mut Vec<ModelInfo>, default_model: &str) {
    if !models
        .iter()
        .any(|model| model.id.eq_ignore_ascii_case(default_model))
    {
        models.push(ModelInfo {
            id: default_model.into(),
            name: default_model.into(),
            context_window: None,
            capabilities: vec!["chat".into()],
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_own_models_are_accepted() {
        assert!(is_owned_model("ernie-5.5-flash-preview", &[]));
        assert!(!is_owned_model("deepseek-chat", &[]));
        assert!(!is_owned_model("gpt-4o", &[]));
        assert!(is_owned_model("my-private", &["my-private".into()]));
    }

    #[test]
    fn builtin_list_contains_default() {
        let models = builtin_models("ernie-5.5-flash-preview");
        assert!(
            models
                .iter()
                .any(|model| model.id == "ernie-5.5-flash-preview")
        );
        assert_eq!(models[0].id, "ernie-5.5-flash-preview");
    }
}
