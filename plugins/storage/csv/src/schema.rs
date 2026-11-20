pub const NOVEL_HEADERS: &[&str] = &[
    "novel_id",
    "title",
    "author",
    "genre",
    "chapter_count",
    "total_words",
    "created_at",
    "updated_at",
];

pub const CONTENT_HEADERS: &[&str] = &[
    "chapter_id",
    "novel_id",
    "chapter_index",
    "fingerprint",
    "chapter_title",
    "word_count",
    "created_at",
    "updated_at",
];

pub const SENTENCE_HEADERS: &[&str] = &[
    "sentence_id",
    "novel_id",
    "chapter_id",
    "sentence_index",
    "content",
    "word_count",
];

pub const DIALOGUE_HEADERS: &[&str] = &[
    "segment_id",
    "novel_id",
    "chapter_id",
    "sentence_id",
    "segment_index",
    "segment_kind",
    "speaker_id",
    "content",
    "word_count",
];

pub const POLYPHONE_HEADERS: &[&str] = &[
    "polyphone_id",
    "segment_id",
    "sentence_id",
    "char_index",
    "character",
    "pinyin",
    "pinyin_base",
    "tone",
    "tts_pinyin",
    "created_at",
    "updated_at",
];

pub const AUDIO_CLIP_HEADERS: &[&str] = &[
    "clip_id",
    "segment_id",
    "sentence_id",
    "chapter_id",
    "novel_id",
    "format",
    "sample_rate",
    "duration_ms",
    "storage_key",
    "tts_engine",
    "voice_id",
    "created_at",
];
