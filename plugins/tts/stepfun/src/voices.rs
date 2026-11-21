//! 内置的阶跃星辰官方音色（拿不到在线列表时兜底）。
//!
//! 完整、最新的音色以接口为准：`GET /v1/audio/system_voices?model=step-tts-2`
//! <https://platform.stepfun.com/docs/zh/api-reference/audio/system-voices>

use leo_core::spi::VoiceInfo;

fn voice(id: &str, name: &str, gender: &str, description: &str) -> VoiceInfo {
    VoiceInfo { id: id.into(), name: name.into(), gender: gender.into(), locale: "zh-CN".into(), description: description.into() }
}

/// 适合有声书的几种官方音色（摘自上面接口文档的返回示例）
pub fn builtin_voices() -> Vec<VoiceInfo> {
    vec![
        voice("cixingnansheng", "磁性男声", "Male", "深情厚重，有感染力，霸总感；有声书、情感陪伴"),
        voice("yuanqinansheng", "元气男声", "Male", "情绪饱满有活力；有声书、口播"),
        voice("zixinnansheng", "自信男声", "Male", "真诚亲和，有活力；有声书、情感陪伴"),
        voice("wenrounansheng", "温柔男声", "Male", "温柔男声"),
        voice("elegantgentle-female", "气质温婉", "Female", "真诚温柔，亲和力强；口播、情感陪伴"),
        voice("linjiameimei", "邻家妹妹", "Female", "可爱亲和，稚气感；视频配音、口播"),
        voice("ruanmengnvsheng", "软萌女声", "Female", "可爱、甜、嗲；情感陪伴、视频配音"),
        voice("shuangkuaijiejie", "爽快姐姐", "Female", "清澈有活力；口播、语音助手"),
    ]
}

/// 由官方描述里的“男，…/女，…”推断性别
pub fn gender_from_description(description: &str) -> &'static str {
    match description.trim_start().chars().next() {
        Some('男') => "Male",
        Some('女') => "Female",
        _ => "",
    }
}
