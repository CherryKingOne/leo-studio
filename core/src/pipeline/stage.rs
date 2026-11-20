use serde::{Deserialize, Serialize};

/// 生产管线的各个阶段状态机
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PipelineStage {
    /// 1. 原始文本刚接入
    Ingested,
    /// 2. 已切分为章节
    Parsed,
    /// 3. 句子与台词/旁白结构拆解完成
    Structured,
    /// 4. 多音字、角色声线与情绪标注完成
    Annotated,
    /// 5. 正在跑 TTS 语音批处理合成
    Synthesizing,
    /// 6. 所有的音频切片全部就绪
    Synthesized,
    /// 7. 母带按时间轴拼接完成
    Assembled,
    /// 8. 导出 MP3/WAV/字幕完成
    Exported,
}

impl PipelineStage {
    pub fn is_audio_ready(&self) -> bool {
        *self >= Self::Synthesized
    }
}
