use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use leo_core::ast::{
    Annotation, AudioClipRef, AudioFormat, ChapterNode, ParagraphNode, ScriptDocument, SegmentKind,
    SegmentNode, SentenceNode,
};
use leo_core::error::{CoreError, CoreResult};
use leo_core::plugin::StorageProvider;

use crate::schema::*;
use crate::table::{CsvTable, Record, int_field, str_field};

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

impl CsvStorageService {
    pub fn new(base_dir: PathBuf) -> CoreResult<Self> {
        let audio_dir = base_dir.join("audio").join("clips");
        fs::create_dir_all(&audio_dir).map_err(|e| {
            CoreError::Storage(format!("创建音频目录 {:?} 失败: {e}", audio_dir))
        })?;

        let tables = CsvTables {
            novels: CsvTable::open(base_dir.join("novels.csv"), NOVEL_HEADERS)?,
            contents: CsvTable::open(base_dir.join("novel_contents.csv"), CONTENT_HEADERS)?,
            sentences: CsvTable::open(base_dir.join("novel_sentences.csv"), SENTENCE_HEADERS)?,
            dialogues: CsvTable::open(base_dir.join("novel_dialogues.csv"), DIALOGUE_HEADERS)?,
            polyphones: CsvTable::open(base_dir.join("novel_polyphones.csv"), POLYPHONE_HEADERS)?,
            audio_clips: CsvTable::open(base_dir.join("novel_audio_clips.csv"), AUDIO_CLIP_HEADERS)?,
        };

        Ok(Self {
            base_dir,
            audio_dir,
            tables: Arc::new(RwLock::new(tables)),
        })
    }
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
        tables.novels.replace_records(novel_rows);

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

        tables.contents.replace_records(content_rows);
        tables.sentences.replace_records(sentence_rows);
        tables.dialogues.replace_records(dialogue_rows);
        tables.polyphones.replace_records(polyphone_rows);
        tables.audio_clips.replace_records(audio_rows);

        // 统一原子落盘
        tables.novels.save()?;
        tables.contents.save()?;
        tables.sentences.save()?;
        tables.dialogues.save()?;
        tables.polyphones.save()?;
        tables.audio_clips.save()?;

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

    async fn save_audio_clip(
        &self,
        clip_id: &str,
        data: &[u8],
        format: AudioFormat,
    ) -> CoreResult<AudioClipRef> {
        let filename = format!("{clip_id}.{}", format.extension());
        let full_path = self.audio_dir.join(&filename);
        let relative_key = format!("audio/clips/{filename}");

        tokio::fs::write(&full_path, data).await.map_err(|err| {
            CoreError::Storage(format!("写入音频文件 {:?} 失败: {err}", full_path))
        })?;

        // 估算时长：默认 MP3 按照 48kbps（6000 字节/秒）估算
        let duration_ms = (data.len() as u64 * 1000) / 6000;

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
        let full_path = self.base_dir.join(storage_key);
        if !full_path.exists() {
            return Ok(None);
        }
        let data = tokio::fs::read(&full_path).await.map_err(|err| {
            CoreError::Storage(format!("读取音频文件 {:?} 失败: {err}", full_path))
        })?;
        Ok(Some(data))
    }
}
