use serde::{Deserialize, Serialize};

/// 挂载在文本/字词节点上的生产注解元数据。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Annotation {
    /// 多音字标注
    Polyphone {
        /// 目标字符在句/片段内的 Unicode 字符偏移（从 0 开始）
        char_index: usize,
        /// 汉字单字
        character: String,
        /// 完整拼音（如 háng）
        pinyin: String,
        /// 拼音基础字母（如 hang）
        pinyin_base: String,
        /// 声调数字 1~5（5 为轻声）
        tone: u8,
        /// 针对 TTS 引擎的标准标注（如 hang2）
        tts_pinyin: String,
    },

    /// 角色说话人绑定
    Speaker {
        speaker_id: String,
        speaker_name: String,
        /// 绑定的声线模型 ID（如 zh-CN-YunxiNeural / cosyvoice:custom_voice_1）
        voice_id: Option<String>,
    },

    /// 情绪/语气标注
    Emotion {
        /// 情绪类别，如 angry, happy, sad, whispered, sarcastic
        emotion_type: String,
        /// 情绪强度 (0.0 ~ 2.0，默认 1.0)
        intensity: f32,
    },

    /// 停顿控制（在片段前或片段后强制停顿）
    Pause {
        position: PausePosition,
        duration_ms: u32,
    },

    /// 局部语速控制 (0.5 ~ 2.0，默认 1.0)
    SpeedRate {
        rate: f32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PausePosition {
    Before,
    After,
}
