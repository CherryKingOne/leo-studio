use chrono::{DateTime, Utc};
use serde::Serialize;

/// 项目元数据。
#[derive(Debug, Clone, Serialize)]
pub struct ProjectInfo {
    pub project_id: String,
    pub name: String,
    pub file_id: String,
    pub filename: String,
    pub status: String,
    pub chapter_count: i64,
    pub total_words: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub is_deleted: bool,
    pub deleted_at: Option<DateTime<Utc>>,
}

/// 章节大纲元数据。
#[derive(Debug, Clone, Serialize)]
pub struct ChapterOutlineInfo {
    pub chapter_id: String,
    pub chapter_index: i64,
    pub chapter_title: String,
    pub word_count: i64,
}

/// 句子切句数据。
#[derive(Debug, Clone, Serialize)]
pub struct ChapterSentenceInfo {
    pub sentence_id: String,
    pub sentence_index: i64,
    pub content: String,
    pub word_count: i64,
    pub polyphones: Vec<SentencePolyphoneInfo>,
}

/// 章节详细内容与切句数据。
#[derive(Debug, Clone, Serialize)]
pub struct ChapterContentInfo {
    pub project_id: String,
    pub file_id: String,
    pub chapter_id: String,
    pub chapter_index: i64,
    pub chapter_title: String,
    pub word_count: i64,
    pub content: String,
    pub sentences: Vec<ChapterSentenceInfo>,
}

/// 章节删除结果。
#[derive(Debug, Clone, Serialize)]
pub struct ChapterDeleteInfo {
    pub project_id: String,
    pub chapter_id: String,
    pub chapter_title: String,
    pub deleted: bool,
    pub chapter_count: i64,
    pub total_words: i64,
    pub updated_at: DateTime<Utc>,
}

/// 章节冲突差异信息。
#[derive(Debug, Clone, Serialize)]
pub struct ChapterConflictInfo {
    pub chapter_id: String,
    pub chapter_index: i64,
    pub chapter_title: String,
    pub existing_word_count: i64,
    pub incoming_word_count: i64,
    pub word_diff: i64,
}

/// 追加章节时单个章节的处理结果。
#[derive(Debug, Clone, Serialize)]
pub struct ChapterAppendItem {
    pub chapter_id: Option<String>,
    pub chapter_index: Option<i64>,
    pub chapter_title: String,
    pub action: String,
    pub word_count: i64,
}

/// 追加章节后的项目统计与结果元数据。
#[derive(Debug, Clone, Serialize)]
pub struct ProjectChapterAppendInfo {
    pub project_id: String,
    pub file_id: String,
    pub dry_run: bool,
    pub added_chapters: i64,
    pub skipped_chapters: i64,
    pub total_chapters: i64,
    pub total_words: i64,
    pub has_conflicts: bool,
    pub conflicts: Vec<ChapterConflictInfo>,
    pub chapters: Vec<ChapterAppendItem>,
    pub updated_at: DateTime<Utc>,
}

/// 句子多音字标注数据。
#[derive(Debug, Clone, Serialize)]
pub struct SentencePolyphoneInfo {
    pub polyphone_id: String,
    pub sentence_id: String,
    pub char_index: i64,
    pub character: String,
    pub pinyin: String,
    pub pinyin_base: String,
    pub tone: i64,
    pub tts_pinyin: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 单条多音字标注删除结果。
#[derive(Debug, Clone, Serialize)]
pub struct SentencePolyphoneItemDeleteInfo {
    pub polyphone_id: String,
    pub sentence_id: String,
    pub char_index: i64,
    pub deleted: bool,
}

/// 章节下某个句子的多音字标注集合。
#[derive(Debug, Clone, Serialize)]
pub struct ChapterPolyphoneSentence {
    pub sentence_id: String,
    pub sentence_index: i64,
    pub content: String,
    pub polyphones: Vec<SentencePolyphoneInfo>,
}

/// 章节多音字标注列表。
#[derive(Debug, Clone, Serialize)]
pub struct ChapterPolyphoneList {
    pub chapter_id: String,
    pub chapter_title: String,
    pub sentence_count: usize,
    pub total: usize,
    pub items: Vec<ChapterPolyphoneSentence>,
}

/// 章节搜索命中项。
#[derive(Debug, Clone, Serialize)]
pub struct ChapterSearchHit {
    pub chapter_id: String,
    pub chapter_index: i64,
    pub chapter_title: String,
    pub word_count: i64,
    /// 命中关键词前后各取若干字的摘要
    pub excerpt: String,
}

/// 全库统计。
#[derive(Debug, Clone, Serialize)]
pub struct ProjectStats {
    pub projects: i64,
    pub chapters: i64,
    pub words: i64,
    pub recycle_bin: i64,
}

pub const STATUS_IN_PROGRESS: &str = "制作中";
