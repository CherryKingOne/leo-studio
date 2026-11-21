//! 音色、情绪、音调换算。
//!
//! - 系统音色列表：<https://platform.minimax.cn/docs/faq/system-voice-id>
//! - 查询可用音色接口：<https://platform.minimax.cn/docs/api-reference/voice-management-get>
//! - 情绪取值见同步语音合成文档 `voice_setting.emotion`：<https://platform.minimax.cn/docs/api-reference/speech-t2a-http>

use leo_core::spi::VoiceInfo;

fn voice(id: &str, name: &str, gender: &str, description: &str) -> VoiceInfo {
    VoiceInfo { id: id.into(), name: name.into(), gender: gender.into(), locale: locale_of(id).into(), description: description.into() }
}

/// 拿不到在线列表时的兜底音色（取自官方文档示例中的系统音色）
pub fn builtin_voices() -> Vec<VoiceInfo> {
    vec![
        voice("male-qn-qingse", "青涩青年音色", "Male", "官方示例音色"),
        voice("Chinese (Mandarin)_Reliable_Executive", "沉稳高管", "Male", "沉稳可靠的中年男性高管声音，标准普通话"),
        voice("Chinese (Mandarin)_News_Anchor", "新闻女声", "Female", "专业、播音腔的中年女性新闻主播，标准普通话"),
        voice("Chinese (Mandarin)_Lyrical_Voice", "抒情音色", "", "官方推荐的最新中文系统音色"),
        voice("Chinese (Mandarin)_HK_Flight_Attendant", "港腔空姐", "Female", "官方推荐的最新中文系统音色"),
    ]
}

/// 由音色 id 前缀推断语言
pub fn locale_of(voice_id: &str) -> &'static str {
    let prefixes: [(&str, &str); 8] = [
        ("Chinese (Mandarin)", "zh-CN"),
        ("Cantonese", "zh-HK"),
        ("English", "en"),
        ("Japanese", "ja"),
        ("Korean", "ko"),
        ("Spanish", "es"),
        ("French", "fr"),
        ("German", "de"),
    ];
    prefixes
        .iter()
        .find(|(p, _)| voice_id.starts_with(p))
        .map(|(_, l)| *l)
        .unwrap_or(if voice_id.starts_with("male-") || voice_id.starts_with("female-") { "zh-CN" } else { "" })
}

/// 由中文描述推断性别（描述里有“男/女”）
pub fn gender_from_text(text: &str) -> &'static str {
    let male = text.contains('男');
    let female = text.contains('女');
    match (male, female) {
        (true, false) => "Male",
        (false, true) => "Female",
        _ => "",
    }
}

/// 把剧本里的情绪（中文或英文）换成 MiniMax 的取值；不认识的返回 None（交给模型自动判断）
pub fn map_emotion(emotion: &str) -> Option<&'static str> {
    let e = emotion.trim().to_lowercase();
    let v = match e.as_str() {
        "happy" | "高兴" | "开心" | "喜悦" => "happy",
        "sad" | "悲伤" | "难过" | "伤心" => "sad",
        "angry" | "愤怒" | "生气" => "angry",
        "fearful" | "害怕" | "恐惧" => "fearful",
        "disgusted" | "厌恶" => "disgusted",
        "surprised" | "惊讶" | "吃惊" => "surprised",
        "calm" | "中性" | "平静" => "calm",
        "fluent" | "生动" => "fluent",
        "whisper" | "低语" | "耳语" => "whisper",
        _ => return None,
    };
    Some(v)
}

/// SynthesisPayload.pitch（-100 ~ +100，百分比）→ MiniMax pitch（-12 ~ 12 半音，整数）
pub fn pitch_to_semitones(pitch: f32) -> i32 {
    ((pitch.clamp(-100.0, 100.0) / 100.0) * 12.0).round() as i32
}
