use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::annotation::Annotation;
use super::audio::AudioClipRef;

/// 片段语义类别：旁白、角色台词、音效
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SegmentKind {
    Narration,
    Dialogue,
    Sfx,
}

/// 最小发音/台词片段
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SegmentNode {
    pub id: String,
    pub index: usize,
    pub kind: SegmentKind,
    pub text: String,
    pub word_count: usize,
    /// 绑定的角色 ID（旁白通常可绑定全局旁白音色）
    pub speaker_id: Option<String>,
    /// 挂载的注音、停顿、情绪等注解
    pub annotations: Vec<Annotation>,
    /// 生成好的切片音频引用（若尚未合成或已被标记为 Dirty，则为 None）
    pub audio: Option<AudioClipRef>,
    /// 是否为脏节点（修改文字或注音后置为 true，待重新合成）
    pub is_dirty: bool,
}

impl SegmentNode {
    pub fn new(id: String, index: usize, kind: SegmentKind, text: String) -> Self {
        let word_count = text.chars().filter(|c| !c.is_whitespace()).count();
        Self {
            id,
            index,
            kind,
            text,
            word_count,
            speaker_id: None,
            annotations: Vec::new(),
            audio: None,
            is_dirty: true,
        }
    }

    /// 标记节点为脏（文本或配置发生变更）
    pub fn mark_dirty(&mut self) {
        self.is_dirty = true;
        self.audio = None;
    }
}

/// 单句节点（通常对应按行切分的句子）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SentenceNode {
    pub id: String,
    pub index: usize,
    pub raw_text: String,
    pub word_count: usize,
    /// 句子内部拆解出的台词与旁白片段列表
    pub segments: Vec<SegmentNode>,
    pub is_dirty: bool,
}

impl SentenceNode {
    pub fn new(id: String, index: usize, raw_text: String) -> Self {
        let word_count = raw_text.chars().filter(|c| !c.is_whitespace()).count();
        Self {
            id,
            index,
            raw_text,
            word_count,
            segments: Vec::new(),
            is_dirty: true,
        }
    }

    pub fn mark_dirty(&mut self) {
        self.is_dirty = true;
        for seg in &mut self.segments {
            seg.mark_dirty();
        }
    }
}

/// 自然段节点
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParagraphNode {
    pub id: String,
    pub index: usize,
    pub sentences: Vec<SentenceNode>,
}

/// 章节节点
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChapterNode {
    pub id: String,
    pub index: usize,
    /// 归一化指纹（如 v_1_ch_12），用于防脏数据覆盖比对
    pub fingerprint: String,
    pub title: String,
    pub paragraphs: Vec<ParagraphNode>,
    pub word_count: usize,
    pub is_dirty: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ChapterNode {
    pub fn new(id: String, index: usize, fingerprint: String, title: String) -> Self {
        let now = Utc::now();
        Self {
            id,
            index,
            fingerprint,
            title,
            paragraphs: Vec::new(),
            word_count: 0,
            is_dirty: true,
            created_at: now,
            updated_at: now,
        }
    }

    /// 重新统计字数并同步状态
    pub fn recalculate_word_count(&mut self) {
        self.word_count = self
            .paragraphs
            .iter()
            .flat_map(|p| &p.sentences)
            .map(|s| s.word_count)
            .sum();
    }
}

/// 分卷节点（可选）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VolumeNode {
    pub id: String,
    pub index: usize,
    pub title: String,
    pub chapters: Vec<ChapterNode>,
}

/// 剧本总文档（根节点）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScriptDocument {
    pub id: String,
    pub title: String,
    pub author: String,
    pub genre: String,
    pub volumes: Vec<VolumeNode>,
    /// 无分卷时的扁平章节列表
    pub chapters: Vec<ChapterNode>,
    pub total_words: usize,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ScriptDocument {
    pub fn new(id: String, title: String, author: String, genre: String) -> Self {
        let now = Utc::now();
        Self {
            id,
            title,
            author,
            genre,
            volumes: Vec::new(),
            chapters: Vec::new(),
            total_words: 0,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn recalculate_totals(&mut self) {
        let flat_words: usize = self.chapters.iter().map(|c| c.word_count).sum();
        let vol_words: usize = self
            .volumes
            .iter()
            .flat_map(|v| &v.chapters)
            .map(|c| c.word_count)
            .sum();
        self.total_words = flat_words + vol_words;
        self.updated_at = Utc::now();
    }
}
