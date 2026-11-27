use std::path::PathBuf;

use crate::config::Settings;
use crate::error::{AppResult, storage_io};

use super::csv_table::CsvTable;

const PROJECTS_FILENAME: &str = "projects.csv";

const NOVEL_HEADERS: &[&str] = &[
    "novel_id",
    "title",
    "format",
    "chapter_count",
    "total_words",
    "line_count",
    "created_at",
];

const CONTENT_HEADERS: &[&str] = &[
    "chapter_id",
    "novel_id",
    "chapter_index",
    "chapter_title",
    "content",
    "word_count",
];

const SENTENCE_HEADERS: &[&str] = &[
    "sentence_id",
    "novel_id",
    "chapter_id",
    "sentence_index",
    "content",
    "word_count",
];

const DIALOGUE_HEADERS: &[&str] = &[
    "dialogue_id",
    "novel_id",
    "chapter_id",
    "parent_sentence_id",
    "source_sentence_index",
    "split_index",
    "segment_type",
    "content",
    "word_count",
    "parse_status",
    "speaker_id",
    "binding_source",
    "binding_confidence",
    "speaker_updated_at",
];

const POLYPHONE_HEADERS: &[&str] = &[
    "polyphone_id",
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

const NOVEL_CHARACTER_HEADERS: &[&str] = &[
    "character_id",
    "project_id",
    "name",
    "aliases_json",
    "gender",
    "age",
    "role_type",
    "status",
    "description",
    "tags_json",
    "created_at",
    "updated_at",
];

const PROJECT_HEADERS: &[&str] = &[
    "project_id",
    "name",
    "file_id",
    "filename",
    "status",
    "chapter_count",
    "total_words",
    "created_at",
    "updated_at",
    "is_deleted",
    "deleted_at",
];

/// 业务数据存储：六张 CSV 表 + 上传目录。
pub struct Store {
    pub upload_dir: PathBuf,
    pub csv_dir: PathBuf,
    pub novels: CsvTable,
    pub contents: CsvTable,
    pub sentences: CsvTable,
    pub dialogues: CsvTable,
    pub polyphones: CsvTable,
    pub characters: CsvTable,
    pub projects: CsvTable,
}

impl Store {
    /// 依据配置初始化存储。
    pub fn new(settings: &Settings) -> AppResult<Self> {
        Self::with_filenames(
            settings.upload_dir.clone(),
            settings.csv_dir.clone(),
            &settings.novels_csv_filename,
            &settings.novel_contents_csv_filename,
            &settings.novel_sentences_csv_filename,
            &settings.novel_dialogues_csv_filename,
            &settings.novel_polyphones_csv_filename,
            &settings.novel_characters_csv_filename,
        )
    }

    /// 以显式文件名初始化存储（便于测试隔离）。
    #[allow(clippy::too_many_arguments)]
    pub fn with_filenames(
        upload_dir: PathBuf,
        csv_dir: PathBuf,
        novels_filename: &str,
        contents_filename: &str,
        sentences_filename: &str,
        dialogues_filename: &str,
        polyphones_filename: &str,
        characters_filename: &str,
    ) -> AppResult<Self> {
        std::fs::create_dir_all(&upload_dir).map_err(|err| storage_io("创建上传目录失败", err))?;
        std::fs::create_dir_all(&csv_dir).map_err(|err| storage_io("创建 CSV 目录失败", err))?;

        Ok(Self {
            novels: CsvTable::open(csv_dir.join(novels_filename), NOVEL_HEADERS)?,
            contents: CsvTable::open(csv_dir.join(contents_filename), CONTENT_HEADERS)?,
            sentences: CsvTable::open(csv_dir.join(sentences_filename), SENTENCE_HEADERS)?,
            dialogues: CsvTable::open(csv_dir.join(dialogues_filename), DIALOGUE_HEADERS)?,
            polyphones: CsvTable::open(csv_dir.join(polyphones_filename), POLYPHONE_HEADERS)?,
            characters: CsvTable::open(csv_dir.join(characters_filename), NOVEL_CHARACTER_HEADERS)?,
            projects: CsvTable::open(csv_dir.join(PROJECTS_FILENAME), PROJECT_HEADERS)?,
            upload_dir,
            csv_dir,
        })
    }

    /// 计算文件在上传目录中的落盘路径。
    pub fn path_for(&self, file_id: &str, extension: &str) -> PathBuf {
        self.upload_dir.join(format!("{file_id}{extension}"))
    }

    pub fn save_novels(&self) -> AppResult<()> {
        self.novels.save()
    }

    pub fn save_contents(&self) -> AppResult<()> {
        self.contents.save()
    }

    pub fn save_sentences(&self) -> AppResult<()> {
        self.sentences.save()
    }

    pub fn save_dialogues(&self) -> AppResult<()> {
        self.dialogues.save()
    }

    pub fn save_polyphones(&self) -> AppResult<()> {
        self.polyphones.save()
    }

    pub fn save_characters(&self) -> AppResult<()> {
        self.characters.save()
    }

    pub fn save_projects(&self) -> AppResult<()> {
        self.projects.save()
    }

    pub fn save_all(&self) -> AppResult<()> {
        self.save_novels()?;
        self.save_contents()?;
        self.save_sentences()?;
        self.save_dialogues()?;
        self.save_polyphones()?;
        self.save_characters()?;
        self.save_projects()
    }
}

/// 将字节数格式化为可读字符串。
pub fn human_size(bytes: usize) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;

    let value = bytes as f64;
    if value >= GB {
        format!("{:.2} GB", value / GB)
    } else if value >= MB {
        format!("{:.2} MB", value / MB)
    } else if value >= KB {
        format!("{:.2} KB", value / KB)
    } else {
        format!("{bytes} B")
    }
}
