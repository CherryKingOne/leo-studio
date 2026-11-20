use std::collections::HashSet;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use leo_core::ast::{
    Annotation, AudioClipRef, AudioFormat, ChapterNode, ParagraphNode, ScriptDocument, SegmentKind,
    SegmentNode, SentenceNode, mp3_duration_ms,
};
use leo_core::error::{CoreError, CoreResult};
use leo_core::plugin::{ScriptSummary, StorageProvider};

use crate::schema::*;
use crate::table::{CsvTable, Record, int_field, save_tables, str_field};

pub struct CsvStorageService {
    base_dir: PathBuf,
    audio_dir: PathBuf,
    tables: Arc<RwLock<CsvTables>>,
}

struct CsvTables {
    novels: CsvTable,
    contents: CsvTable,
    sentences: CsvTable,
    dialogues: CsvTable,
    polyphones: CsvTable,
    audio_clips: CsvTable,
}

/// 插件表所在子目录，与 REST Store 的同名 CSV 表物理隔离，避免互相整表覆盖。
pub const TABLE_DIR: &str = "scripts";

const NOVELS_FILE: &str = "novels.csv";
const CONTENTS_FILE: &str = "novel_contents.csv";
const SENTENCES_FILE: &str = "novel_sentences.csv";
const DIALOGUES_FILE: &str = "novel_dialogues.csv";
const POLYPHONES_FILE: &str = "novel_polyphones.csv";
const AUDIO_CLIPS_FILE: &str = "novel_audio_clips.csv";

impl CsvStorageService {
    pub fn new(base_dir: PathBuf) -> CoreResult<Self> {
        let audio_dir = base_dir.join("audio").join("clips");
        fs::create_dir_all(&audio_dir).map_err(|e| {
            CoreError::Storage(format!("创建音频目录 {:?} 失败: {e}", audio_dir))
        })?;

        let table_dir = base_dir.join(TABLE_DIR);
        let first_open = !table_dir.join(NOVELS_FILE).exists();

        let mut tables = CsvTables {
            novels: CsvTable::open(table_dir.join(NOVELS_FILE), NOVEL_HEADERS)?,
            contents: CsvTable::open(table_dir.join(CONTENTS_FILE), CONTENT_HEADERS)?,
            sentences: CsvTable::open(table_dir.join(SENTENCES_FILE), SENTENCE_HEADERS)?,
            dialogues: CsvTable::open(table_dir.join(DIALOGUES_FILE), DIALOGUE_HEADERS)?,
            polyphones: CsvTable::open(table_dir.join(POLYPHONES_FILE), POLYPHONE_HEADERS)?,
            audio_clips: CsvTable::open(table_dir.join(AUDIO_CLIPS_FILE), AUDIO_CLIP_HEADERS)?,
        };

        if first_open {
            migrate_legacy_tables(&base_dir, &mut tables)?;
            tables.save_all()?;
        }

        Ok(Self {
            base_dir,
            audio_dir,
            tables: Arc::new(RwLock::new(tables)),
        })
    }
}

impl CsvTables {
    /// 六张表两阶段落盘：全部写好临时文件并 fsync 后才统一替换，任一表写失败则一张都不改。
    fn save_all(&self) -> CoreResult<()> {
        save_tables(&[
            &self.novels,
            &self.contents,
            &self.sentences,
            &self.dialogues,
            &self.polyphones,
            &self.audio_clips,
        ])
    }
}

/// 旧版本把插件表直接写在 `base_dir` 下，与 REST Store 共用文件名。
/// 首次启用独立子目录时，只把插件写入的行（novels 表带 `updated_at`）迁移过来，旧文件保持不动。
fn migrate_legacy_tables(base_dir: &Path, tables: &mut CsvTables) -> CoreResult<()> {
    let legacy_novels_path = base_dir.join(NOVELS_FILE);
    if legacy_novels_path.exists() {
        let legacy_novels = CsvTable::open(legacy_novels_path, NOVEL_HEADERS)?;
        let novel_ids: HashSet<String> = legacy_novels
            .records()
            .iter()
            .filter(|r| !str_field(r, "updated_at").is_empty())
            .map(|r| str_field(r, "novel_id").to_string())
            .collect();

        if !novel_ids.is_empty() {
            let owned = |r: &Record| novel_ids.contains(str_field(r, "novel_id"));
            tables.novels.replace_records(
                legacy_novels.records().iter().filter(|r| owned(r)).cloned().collect(),
            );
            tables
                .contents
                .replace_records(legacy_rows(base_dir, CONTENTS_FILE, CONTENT_HEADERS, owned)?);
            tables
                .sentences
                .replace_records(legacy_rows(base_dir, SENTENCES_FILE, SENTENCE_HEADERS, owned)?);
            tables
                .dialogues
                .replace_records(legacy_rows(base_dir, DIALOGUES_FILE, DIALOGUE_HEADERS, owned)?);

            let sentence_ids: HashSet<String> = tables
                .sentences
                .records()
                .iter()
                .map(|r| str_field(r, "sentence_id").to_string())
                .collect();
            tables.polyphones.replace_records(legacy_rows(
                base_dir,
                POLYPHONES_FILE,
                POLYPHONE_HEADERS,
                |r| sentence_ids.contains(str_field(r, "sentence_id")),
            )?);
        }
    }

    tables.audio_clips.replace_records(legacy_rows(
        base_dir,
        AUDIO_CLIPS_FILE,
        AUDIO_CLIP_HEADERS,
        |_| true,
    )?);
    Ok(())
}

fn legacy_rows(
    base_dir: &Path,
    filename: &str,
    headers: &[&str],
    keep: impl Fn(&Record) -> bool,
) -> CoreResult<Vec<Record>> {
    let path = base_dir.join(filename);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let table = CsvTable::open(path, headers)?;
    Ok(table.records().iter().filter(|r| keep(r)).cloned().collect())
}

#[async_trait]
impl StorageProvider for CsvStorageService {
    async fn save_script(&self, script: &ScriptDocument) -> CoreResult<()> {
        let mut tables = self.tables.write().await;

        // 1. 更新 novels 表
        let mut novel_record = Record::new();
        novel_record.insert("novel_id".into(), script.id.clone());
        novel_record.insert("title".into(), script.title.clone());
        novel_record.insert("author".into(), script.author.clone());
        novel_record.insert("genre".into(), script.genre.clone());
        novel_record.insert("chapter_count".into(), script.chapters.len().to_string());
        novel_record.insert("total_words".into(), script.total_words.to_string());
        novel_record.insert("created_at".into(), script.created_at.to_rfc3339());
        novel_record.insert("updated_at".into(), script.updated_at.to_rfc3339());

        let mut novel_rows: Vec<Record> = tables
            .novels
            .records()
            .iter()
            .filter(|r| str_field(r, "novel_id") != script.id)
            .cloned()
            .collect();
        novel_rows.push(novel_record);

        // 2. 更新 contents, sentences, dialogues, polyphones, audio_clips
        let mut content_rows: Vec<Record> = tables
            .contents
            .records()
            .iter()
            .filter(|r| str_field(r, "novel_id") != script.id)
            .cloned()
            .collect();
        let mut sentence_rows: Vec<Record> = tables
            .sentences
            .records()
            .iter()
            .filter(|r| str_field(r, "novel_id") != script.id)
            .cloned()
            .collect();
        let mut dialogue_rows: Vec<Record> = tables
            .dialogues
            .records()
            .iter()
            .filter(|r| str_field(r, "novel_id") != script.id)
            .cloned()
            .collect();
        let mut polyphone_rows: Vec<Record> = tables
            .polyphones
            .records()
            .iter()
            .filter(|r| {
                // 保留不属于当前剧目的标注
                let sent_id = str_field(r, "sentence_id");
                !tables
                    .sentences
                    .records()
                    .iter()
                    .any(|s| str_field(s, "novel_id") == script.id && str_field(s, "sentence_id") == sent_id)
            })
            .cloned()
            .collect();
        let mut audio_rows: Vec<Record> = tables
            .audio_clips
            .records()
            .iter()
            .filter(|r| str_field(r, "novel_id") != script.id)
            .cloned()
            .collect();

        for chapter in &script.chapters {
            let mut c_rec = Record::new();
            c_rec.insert("chapter_id".into(), chapter.id.clone());
            c_rec.insert("novel_id".into(), script.id.clone());
            c_rec.insert("chapter_index".into(), chapter.index.to_string());
            c_rec.insert("fingerprint".into(), chapter.fingerprint.clone());
            c_rec.insert("chapter_title".into(), chapter.title.clone());
            c_rec.insert("word_count".into(), chapter.word_count.to_string());
            c_rec.insert("created_at".into(), chapter.created_at.to_rfc3339());
            c_rec.insert("updated_at".into(), chapter.updated_at.to_rfc3339());
            content_rows.push(c_rec);

            for paragraph in &chapter.paragraphs {
                for sentence in &paragraph.sentences {
                    let mut s_rec = Record::new();
                    s_rec.insert("sentence_id".into(), sentence.id.clone());
                    s_rec.insert("novel_id".into(), script.id.clone());
                    s_rec.insert("chapter_id".into(), chapter.id.clone());
                    s_rec.insert("sentence_index".into(), sentence.index.to_string());
                    s_rec.insert("content".into(), sentence.raw_text.clone());
                    s_rec.insert("word_count".into(), sentence.word_count.to_string());
                    sentence_rows.push(s_rec);

                    for segment in &sentence.segments {
                        let mut d_rec = Record::new();
                        d_rec.insert("segment_id".into(), segment.id.clone());
                        d_rec.insert("novel_id".into(), script.id.clone());
                        d_rec.insert("chapter_id".into(), chapter.id.clone());
                        d_rec.insert("sentence_id".into(), sentence.id.clone());
                        d_rec.insert("segment_index".into(), segment.index.to_string());
                        d_rec.insert(
                            "segment_kind".into(),
                            match segment.kind {
                                SegmentKind::Narration => "narration".into(),
                                SegmentKind::Dialogue => "dialogue".into(),
                                SegmentKind::Sfx => "sfx".into(),
                            },
                        );
                        d_rec.insert(
                            "speaker_id".into(),
                            segment.speaker_id.clone().unwrap_or_default(),
                        );
                        d_rec.insert("content".into(), segment.text.clone());
                        d_rec.insert("word_count".into(), segment.word_count.to_string());
                        dialogue_rows.push(d_rec);

                        // 提取多音字注解
                        for ann in &segment.annotations {
                            if let Annotation::Polyphone {
                                char_index,
                                character,
                                pinyin,
                                pinyin_base,
                                tone,
                                tts_pinyin,
                            } = ann
                            {
                                let mut p_rec = Record::new();
                                p_rec.insert(
                                    "polyphone_id".into(),
                                    uuid::Uuid::new_v4().simple().to_string(),
                                );
                                p_rec.insert("segment_id".into(), segment.id.clone());
                                p_rec.insert("sentence_id".into(), sentence.id.clone());
                                p_rec.insert("char_index".into(), char_index.to_string());
                                p_rec.insert("character".into(), character.clone());
                                p_rec.insert("pinyin".into(), pinyin.clone());
                                p_rec.insert("pinyin_base".into(), pinyin_base.clone());
                                p_rec.insert("tone".into(), tone.to_string());
                                p_rec.insert("tts_pinyin".into(), tts_pinyin.clone());
                                p_rec.insert("created_at".into(), Utc::now().to_rfc3339());
                                p_rec.insert("updated_at".into(), Utc::now().to_rfc3339());
                                polyphone_rows.push(p_rec);
                            }
                        }

                        // 保存音频切片元数据
                        if let Some(audio) = &segment.audio {
                            let mut a_rec = Record::new();
                            a_rec.insert("clip_id".into(), audio.clip_id.clone());
                            a_rec.insert("segment_id".into(), segment.id.clone());
                            a_rec.insert("sentence_id".into(), sentence.id.clone());
                            a_rec.insert("chapter_id".into(), chapter.id.clone());
                            a_rec.insert("novel_id".into(), script.id.clone());
                            a_rec.insert("format".into(), audio.format.extension().into());
                            a_rec.insert("sample_rate".into(), audio.sample_rate.to_string());
                            a_rec.insert("duration_ms".into(), audio.duration_ms.to_string());
                            a_rec.insert("storage_key".into(), audio.storage_key.clone());
                            a_rec.insert("tts_engine".into(), audio.tts_engine.clone());
                            a_rec.insert("voice_id".into(), audio.voice_id.clone());
                            a_rec.insert("created_at".into(), audio.created_at.to_rfc3339());
                            audio_rows.push(a_rec);
                        }
                    }
                }
            }
        }

        // 先把新内容写到磁盘（两阶段、全部成功才替换），成功后再更新内存中的表；
        // 落盘失败时内存保持旧状态，与磁盘一致，后续读取不会看到“未保存”的数据。
        let staged = CsvTables {
            novels: tables.novels.with_records(novel_rows),
            contents: tables.contents.with_records(content_rows),
            sentences: tables.sentences.with_records(sentence_rows),
            dialogues: tables.dialogues.with_records(dialogue_rows),
            polyphones: tables.polyphones.with_records(polyphone_rows),
            audio_clips: tables.audio_clips.with_records(audio_rows),
        };
        staged.save_all()?;
        *tables = staged;
        Ok(())
    }

    async fn load_script(&self, project_id: &str) -> CoreResult<Option<ScriptDocument>> {
        let tables = self.tables.read().await;

        let novel_row = match tables
            .novels
            .records()
            .iter()
            .find(|r| str_field(r, "novel_id") == project_id)
        {
            Some(row) => row,
            None => return Ok(None),
        };

        let title = str_field(novel_row, "title").to_string();
        let author = str_field(novel_row, "author").to_string();
        let genre = str_field(novel_row, "genre").to_string();
        let created_at = DateTime::parse_from_rfc3339(str_field(novel_row, "created_at"))
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());
        let updated_at = DateTime::parse_from_rfc3339(str_field(novel_row, "updated_at"))
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        let mut script = ScriptDocument::new(project_id.to_string(), title, author, genre);
        script.created_at = created_at;
        script.updated_at = updated_at;

        // 加载章节
        let mut chapter_rows: Vec<_> = tables
            .contents
            .records()
            .iter()
            .filter(|r| str_field(r, "novel_id") == project_id)
            .collect();
        chapter_rows.sort_by_key(|r| int_field(r, "chapter_index"));

        for c_row in chapter_rows {
            let chapter_id = str_field(c_row, "chapter_id").to_string();
            let chapter_index = int_field(c_row, "chapter_index") as usize;
            let fingerprint = str_field(c_row, "fingerprint").to_string();
            let chapter_title = str_field(c_row, "chapter_title").to_string();

            let mut chapter_node =
                ChapterNode::new(chapter_id.clone(), chapter_index, fingerprint, chapter_title);

            // 加载句子
            let mut sentence_rows: Vec<_> = tables
                .sentences
                .records()
                .iter()
                .filter(|r| str_field(r, "chapter_id") == chapter_id)
                .collect();
            sentence_rows.sort_by_key(|r| int_field(r, "sentence_index"));

            let mut paragraph = ParagraphNode {
                id: format!("p_{chapter_id}"),
                index: 0,
                sentences: Vec::new(),
            };

            for s_row in sentence_rows {
                let sentence_id = str_field(s_row, "sentence_id").to_string();
                let sentence_index = int_field(s_row, "sentence_index") as usize;
                let content = str_field(s_row, "content").to_string();

                let mut sentence_node =
                    SentenceNode::new(sentence_id.clone(), sentence_index, content);

                // 加载片段
                let mut dialogue_rows: Vec<_> = tables
                    .dialogues
                    .records()
                    .iter()
                    .filter(|r| str_field(r, "sentence_id") == sentence_id)
                    .collect();
                dialogue_rows.sort_by_key(|r| int_field(r, "segment_index"));

                for d_row in dialogue_rows {
                    let segment_id = str_field(d_row, "segment_id").to_string();
                    let segment_index = int_field(d_row, "segment_index") as usize;
                    let segment_text = str_field(d_row, "content").to_string();
                    let kind_str = str_field(d_row, "segment_kind");
                    let kind = match kind_str {
                        "dialogue" => SegmentKind::Dialogue,
                        "sfx" => SegmentKind::Sfx,
                        _ => SegmentKind::Narration,
                    };

                    let mut segment_node =
                        SegmentNode::new(segment_id.clone(), segment_index, kind, segment_text);
                    let speaker_id = str_field(d_row, "speaker_id");
                    if !speaker_id.is_empty() {
                        segment_node.speaker_id = Some(speaker_id.to_string());
                    }

                    // 挂载多音字标注
                    let mut polyphone_rows: Vec<_> = tables
                        .polyphones
                        .records()
                        .iter()
                        .filter(|r| str_field(r, "segment_id") == segment_id)
                        .collect();
                    polyphone_rows.sort_by_key(|r| int_field(r, "char_index"));
                    for p_row in polyphone_rows {
                        segment_node.annotations.push(Annotation::Polyphone {
                            char_index: int_field(p_row, "char_index").max(0) as usize,
                            character: str_field(p_row, "character").to_string(),
                            pinyin: str_field(p_row, "pinyin").to_string(),
                            pinyin_base: str_field(p_row, "pinyin_base").to_string(),
                            tone: int_field(p_row, "tone").clamp(0, u8::MAX as i64) as u8,
                            tts_pinyin: str_field(p_row, "tts_pinyin").to_string(),
                        });
                    }

                    // 挂载音频
                    if let Some(a_row) = tables
                        .audio_clips
                        .records()
                        .iter()
                        .find(|r| str_field(r, "segment_id") == segment_id)
                    {
                        segment_node.audio = Some(AudioClipRef {
                            clip_id: str_field(a_row, "clip_id").to_string(),
                            format: match str_field(a_row, "format") {
                                "wav" => AudioFormat::Wav,
                                "opus" => AudioFormat::Opus,
                                "flac" => AudioFormat::Flac,
                                _ => AudioFormat::Mp3,
                            },
                            sample_rate: int_field(a_row, "sample_rate") as u32,
                            duration_ms: int_field(a_row, "duration_ms") as u64,
                            storage_key: str_field(a_row, "storage_key").to_string(),
                            tts_engine: str_field(a_row, "tts_engine").to_string(),
                            voice_id: str_field(a_row, "voice_id").to_string(),
                            created_at: DateTime::parse_from_rfc3339(str_field(
                                a_row,
                                "created_at",
                            ))
                            .map(|dt| dt.with_timezone(&Utc))
                            .unwrap_or_else(|_| Utc::now()),
                        });
                        segment_node.is_dirty = false;
                    }

                    sentence_node.segments.push(segment_node);
                }

                paragraph.sentences.push(sentence_node);
            }

            chapter_node.paragraphs.push(paragraph);
            chapter_node.recalculate_word_count();
            script.chapters.push(chapter_node);
        }

        script.recalculate_totals();
        Ok(Some(script))
    }

    async fn list_scripts(&self) -> CoreResult<Vec<ScriptSummary>> {
        let tables = self.tables.read().await;
        let mut summaries: Vec<ScriptSummary> = tables
            .novels
            .records()
            .iter()
            .map(|r| ScriptSummary {
                id: str_field(r, "novel_id").to_string(),
                title: str_field(r, "title").to_string(),
                author: str_field(r, "author").to_string(),
                chapter_count: int_field(r, "chapter_count").max(0) as usize,
                total_words: int_field(r, "total_words").max(0) as usize,
                updated_at: str_field(r, "updated_at").to_string(),
            })
            .collect();
        // 最近更新的排在前面
        summaries.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(summaries)
    }

    async fn save_audio_clip(
        &self,
        clip_id: &str,
        data: &[u8],
        format: AudioFormat,
    ) -> CoreResult<AudioClipRef> {
        if !is_safe_clip_id(clip_id) {
            return Err(CoreError::InvalidParameter(format!("非法的音频切片 ID: {clip_id:?}")));
        }
        let filename = format!("{clip_id}.{}", format.extension());
        let full_path = self.audio_dir.join(&filename);
        let relative_key = format!("audio/clips/{filename}");
        if let Ok(meta) = tokio::fs::symlink_metadata(&full_path).await
            && meta.file_type().is_symlink()
        {
            return Err(CoreError::InvalidParameter(format!(
                "音频文件 {:?} 是符号链接，拒绝写入",
                full_path
            )));
        }

        tokio::fs::write(&full_path, data).await.map_err(|err| {
            CoreError::Storage(format!("写入音频文件 {:?} 失败: {err}", full_path))
        })?;

        // MP3 按帧头计算真实时长；其它格式或无法解析时按 48kbps（6000 字节/秒）估算
        let estimated = (data.len() as u64 * 1000) / 6000;
        let duration_ms = match format {
            AudioFormat::Mp3 => mp3_duration_ms(data).unwrap_or(estimated),
            _ => estimated,
        };

        Ok(AudioClipRef {
            clip_id: clip_id.to_string(),
            format,
            sample_rate: 24000,
            duration_ms,
            storage_key: relative_key,
            tts_engine: "default".into(),
            voice_id: "default".into(),
            created_at: Utc::now(),
        })
    }

    async fn load_audio_clip(&self, storage_key: &str) -> CoreResult<Option<Vec<u8>>> {
        let relative = Path::new(storage_key);
        if storage_key.is_empty()
            || !relative
                .components()
                .all(|c| matches!(c, Component::Normal(_)))
        {
            return Err(CoreError::InvalidParameter(format!("非法的音频存储键: {storage_key:?}")));
        }
        let full_path = self.base_dir.join(relative);
        if !full_path.exists() {
            return Ok(None);
        }
        let resolved = full_path.canonicalize().map_err(|err| {
            CoreError::Storage(format!("解析音频路径 {:?} 失败: {err}", full_path))
        })?;
        let audio_root = self.audio_dir.canonicalize().map_err(|err| {
            CoreError::Storage(format!("解析音频目录 {:?} 失败: {err}", self.audio_dir))
        })?;
        if !resolved.starts_with(&audio_root) {
            return Err(CoreError::InvalidParameter(format!(
                "音频存储键越出音频目录: {storage_key:?}"
            )));
        }
        let data = tokio::fs::read(&resolved).await.map_err(|err| {
            CoreError::Storage(format!("读取音频文件 {:?} 失败: {err}", full_path))
        })?;
        Ok(Some(data))
    }
}

/// 音频切片 ID 只允许 ASCII 字母、数字、`_` 与 `-`，杜绝任何路径分隔或 `..`。
pub fn is_safe_clip_id(clip_id: &str) -> bool {
    !clip_id.is_empty()
        && clip_id.len() <= 128
        && clip_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}
