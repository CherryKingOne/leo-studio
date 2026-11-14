use std::collections::HashMap;

use chrono::{DateTime, Utc};

use crate::config::Settings;
use crate::error::{AppError, AppResult};
use crate::modules::text_files::service as text_service;
use crate::storage::csv_table::{Record, bool_field, int_field, str_field};
use crate::storage::store::Store;
use crate::text::parsers::{
    chapter_number, clean_text, count_words, parse_chapters, resolve_chapter_key,
};
use crate::text::processor::create_sentence_records;

use super::model::{
    ChapterAppendItem, ChapterConflictInfo, ChapterContentInfo, ChapterDeleteInfo,
    ChapterOutlineInfo, ChapterPolyphoneList, ChapterPolyphoneSentence, ChapterSearchHit,
    ChapterSentenceInfo, ProjectChapterAppendInfo, ProjectInfo, ProjectStats, STATUS_IN_PROGRESS,
    SentencePolyphoneInfo, SentencePolyphoneItemDeleteInfo,
};

/// 将 CSV 记录转换为项目领域模型。
pub fn record_to_info(record: &Record) -> ProjectInfo {
    let created_at = text_service::parse_datetime(str_field(record, "created_at"));
    let updated_at_raw = str_field(record, "updated_at");
    let updated_at = if updated_at_raw.is_empty() {
        created_at
    } else {
        text_service::parse_datetime(updated_at_raw)
    };

    let is_deleted = bool_field(record, "is_deleted");
    let deleted_at_raw = str_field(record, "deleted_at");
    let deleted_at = if deleted_at_raw.is_empty() || !is_deleted {
        None
    } else {
        Some(text_service::parse_datetime(deleted_at_raw))
    };

    ProjectInfo {
        project_id: str_field(record, "project_id").to_string(),
        name: str_field(record, "name").to_string(),
        file_id: str_field(record, "file_id").to_string(),
        filename: str_field(record, "filename").to_string(),
        status: {
            let status = str_field(record, "status");
            if status.is_empty() {
                STATUS_IN_PROGRESS.to_string()
            } else {
                status.to_string()
            }
        },
        chapter_count: int_field(record, "chapter_count"),
        total_words: int_field(record, "total_words"),
        created_at,
        updated_at,
        is_deleted,
        deleted_at,
    }
}

/// 将 CSV 记录转换为多音字标注领域模型。
pub fn record_to_polyphone_info(record: &Record) -> SentencePolyphoneInfo {
    let created_at = text_service::parse_datetime(str_field(record, "created_at"));
    let updated_at_raw = str_field(record, "updated_at");
    let updated_at = if updated_at_raw.is_empty() {
        created_at
    } else {
        text_service::parse_datetime(updated_at_raw)
    };

    let raw_pinyin = str_field(record, "pinyin");
    let (derived_base, derived_tone, derived_tts) =
        crate::text::parsers::parse_pinyin_elements(raw_pinyin);

    let pinyin_base = {
        let value = str_field(record, "pinyin_base");
        if value.is_empty() {
            derived_base.clone()
        } else {
            value.to_string()
        }
    };
    let tone_raw = str_field(record, "tone");
    let tone = tone_raw
        .trim()
        .parse::<i64>()
        .unwrap_or(derived_tone as i64);
    let tts_pinyin = {
        let value = str_field(record, "tts_pinyin");
        if value.is_empty() {
            derived_tts.clone()
        } else {
            value.to_string()
        }
    };

    SentencePolyphoneInfo {
        polyphone_id: str_field(record, "polyphone_id").to_string(),
        sentence_id: str_field(record, "sentence_id").to_string(),
        char_index: int_field(record, "char_index"),
        character: str_field(record, "character").to_string(),
        pinyin: raw_pinyin.to_string(),
        pinyin_base,
        tone,
        tts_pinyin,
        created_at,
        updated_at,
    }
}

/// 创建项目并上传小说文本文件。
pub fn create_project(
    store: &mut Store,
    settings: &Settings,
    name: &str,
    filename: &str,
    content: &[u8],
    content_type: Option<&str>,
) -> AppResult<ProjectInfo> {
    let clean_name = name.trim();
    if clean_name.is_empty() {
        return Err(AppError::invalid_parameter("项目名称不能为空"));
    }
    if filename.trim().is_empty() {
        return Err(AppError::invalid_parameter("上传文件不能为空"));
    }

    let file_info =
        text_service::save_file(store, settings, filename, content, content_type, None)?;

    let (mut chapter_count, mut total_words) = (1i64, file_info.size as i64);
    if let Some(row) = store.novels.find_by("novel_id", &file_info.file_id) {
        let chapter_count_value = int_field(row, "chapter_count");
        let total_words_value = int_field(row, "total_words");
        if chapter_count_value > 0 {
            chapter_count = chapter_count_value;
        }
        if total_words_value > 0 {
            total_words = total_words_value;
        }
    }

    store.novels.update_by(
        "novel_id",
        &file_info.file_id,
        &[("title", clean_name.to_string())],
    );
    store.save_novels()?;

    let project_id = uuid::Uuid::new_v4().simple().to_string();
    let now = Utc::now();
    let now_iso = now.to_rfc3339();

    let mut record = Record::new();
    record.insert("project_id".into(), project_id.clone());
    record.insert("name".into(), clean_name.to_string());
    record.insert("file_id".into(), file_info.file_id.clone());
    record.insert("filename".into(), file_info.filename.clone());
    record.insert("status".into(), STATUS_IN_PROGRESS.to_string());
    record.insert("chapter_count".into(), chapter_count.to_string());
    record.insert("total_words".into(), total_words.to_string());
    record.insert("created_at".into(), now_iso.clone());
    record.insert("updated_at".into(), now_iso);
    record.insert("is_deleted".into(), "false".to_string());
    record.insert("deleted_at".into(), String::new());

    store.projects.insert(record.clone());
    store.save_projects()?;
    Ok(record_to_info(&record))
}

/// 根据 project_id 查询项目详情。
pub fn get_project(
    store: &Store,
    project_id: &str,
    include_deleted: bool,
) -> AppResult<ProjectInfo> {
    let record = store
        .projects
        .find_by("project_id", project_id)
        .ok_or_else(|| AppError::file_not_found(format!("项目不存在：{project_id}")))?;
    let info = record_to_info(record);
    if !include_deleted && info.is_deleted {
        return Err(AppError::file_not_found(format!(
            "项目不存在或已移入回收站：{project_id}"
        )));
    }
    Ok(info)
}

/// 分页获取项目列表。
pub fn list_projects(
    store: &Store,
    page: usize,
    page_size: usize,
    in_recycle_bin: bool,
) -> (Vec<ProjectInfo>, usize) {
    let mut items: Vec<ProjectInfo> = store
        .projects
        .find_all()
        .iter()
        .map(record_to_info)
        .collect();

    if in_recycle_bin {
        items.retain(|item| item.is_deleted);
        items.sort_by(|a, b| {
            let a_key = a.deleted_at.unwrap_or(a.updated_at);
            let b_key = b.deleted_at.unwrap_or(b.updated_at);
            b_key.cmp(&a_key)
        });
    } else {
        items.retain(|item| !item.is_deleted);
        items.sort_by_key(|item| std::cmp::Reverse(item.created_at));
    }

    let total = items.len();
    let start = page.saturating_sub(1).saturating_mul(page_size);
    let items = items.into_iter().skip(start).take(page_size).collect();
    (items, total)
}

/// 更新项目名称（重命名）。
pub fn update_project(
    store: &mut Store,
    project_id: &str,
    name: Option<&str>,
) -> AppResult<ProjectInfo> {
    let actual_name = name.unwrap_or("").trim();
    if actual_name.is_empty() {
        return Err(AppError::invalid_parameter("项目名称不能为空"));
    }
    if actual_name.chars().count() > 255 {
        return Err(AppError::invalid_parameter(
            "项目名称长度不能超过 255 个字符",
        ));
    }

    get_project(store, project_id, false)?;
    let now = Utc::now();
    store.projects.update_by(
        "project_id",
        project_id,
        &[
            ("name", actual_name.to_string()),
            ("updated_at", now.to_rfc3339()),
        ],
    );
    store.save_projects()?;
    get_project(store, project_id, false)
}

/// 软删除项目：将其移入回收站，保留小说及切句数据支持随时恢复。
pub fn delete_project(store: &mut Store, project_id: &str) -> AppResult<ProjectInfo> {
    let info = get_project(store, project_id, false)?;
    let now = Utc::now();
    let now_iso = now.to_rfc3339();

    store.projects.update_by(
        "project_id",
        project_id,
        &[
            ("is_deleted", "true".to_string()),
            ("deleted_at", now_iso.clone()),
            ("updated_at", now_iso),
        ],
    );
    store.save_projects()?;

    Ok(ProjectInfo {
        project_id: info.project_id,
        name: info.name,
        file_id: info.file_id,
        filename: info.filename,
        status: info.status,
        chapter_count: info.chapter_count,
        total_words: info.total_words,
        created_at: info.created_at,
        updated_at: now,
        is_deleted: true,
        deleted_at: Some(now),
    })
}

/// 从回收站恢复项目。
pub fn restore_project(store: &mut Store, project_id: &str) -> AppResult<ProjectInfo> {
    let info = get_project(store, project_id, true)?;
    if !info.is_deleted {
        return Err(AppError::invalid_parameter(format!(
            "项目未处于回收站中：{project_id}"
        )));
    }

    let now = Utc::now();
    store.projects.update_by(
        "project_id",
        project_id,
        &[
            ("is_deleted", "false".to_string()),
            ("deleted_at", String::new()),
            ("updated_at", now.to_rfc3339()),
        ],
    );
    store.save_projects()?;

    Ok(ProjectInfo {
        project_id: info.project_id,
        name: info.name,
        file_id: info.file_id,
        filename: info.filename,
        status: info.status,
        chapter_count: info.chapter_count,
        total_words: info.total_words,
        created_at: info.created_at,
        updated_at: now,
        is_deleted: false,
        deleted_at: None,
    })
}

/// 彻底物理删除项目及其关联的文本文件、章节与切句数据。
pub fn hard_delete_project(store: &mut Store, project_id: &str) -> AppResult<ProjectInfo> {
    let info = get_project(store, project_id, true)?;
    if !info.is_deleted {
        return Err(AppError::invalid_parameter(format!(
            "项目未处于回收站中：{project_id}"
        )));
    }
    store.projects.delete_by("project_id", project_id);
    store.save_projects()?;

    if let Err(err) = text_service::delete_file(store, &info.file_id)
        && err.code != crate::error::ErrorCode::FILE_NOT_FOUND
    {
        return Err(err);
    }
    Ok(info)
}

/// 清空回收站中所有被软删除的项目。
pub fn clear_recycle_bin(store: &mut Store) -> AppResult<usize> {
    let items: Vec<ProjectInfo> = store
        .projects
        .find_all()
        .iter()
        .map(record_to_info)
        .filter(|item| item.is_deleted)
        .collect();

    let mut cleared_count = 0;
    for item in items {
        store.projects.delete_by("project_id", &item.project_id);
        if let Err(err) = text_service::delete_file(store, &item.file_id)
            && err.code != crate::error::ErrorCode::FILE_NOT_FOUND
        {
            return Err(err);
        }
        cleared_count += 1;
    }
    store.save_projects()?;
    Ok(cleared_count)
}

/// 获取指定项目的章节大纲列表。
pub fn get_project_chapters(store: &Store, project_id: &str) -> AppResult<Vec<ChapterOutlineInfo>> {
    let project = get_project(store, project_id, false)?;
    let mut items: Vec<ChapterOutlineInfo> =
        text_service::get_novel_chapters(store, &project.file_id)
            .iter()
            .map(|row| ChapterOutlineInfo {
                chapter_id: str_field(row, "chapter_id").to_string(),
                chapter_index: int_field(row, "chapter_index"),
                chapter_title: str_field(row, "chapter_title").to_string(),
                word_count: int_field(row, "word_count"),
            })
            .collect();
    items.sort_by_key(|item| item.chapter_index);
    Ok(items)
}

/// 获取指定项目的单个章节详细内容及切句数据。
pub fn get_project_chapter_content(
    store: &Store,
    project_id: &str,
    chapter_id: &str,
) -> AppResult<ChapterContentInfo> {
    let project = get_project(store, project_id, false)?;
    let chapter_row = store
        .contents
        .find_by("chapter_id", chapter_id)
        .filter(|row| str_field(row, "novel_id") == project.file_id)
        .ok_or_else(|| AppError::file_not_found(format!("章节不存在：{chapter_id}")))?;

    let mut sentence_rows: Vec<Record> = text_service::get_chapter_sentences(store, chapter_id);
    sentence_rows.sort_by_key(|row| int_field(row, "sentence_index"));

    let sentence_ids: Vec<String> = sentence_rows
        .iter()
        .map(|row| str_field(row, "sentence_id").to_string())
        .collect();

    let mut polyphones_map: HashMap<String, Vec<SentencePolyphoneInfo>> = HashMap::new();
    for row in store.polyphones.find_all() {
        let sentence_id = str_field(row, "sentence_id");
        if sentence_ids.iter().any(|id| id.as_str() == sentence_id) {
            polyphones_map
                .entry(sentence_id.to_string())
                .or_default()
                .push(record_to_polyphone_info(row));
        }
    }
    for items in polyphones_map.values_mut() {
        items.sort_by_key(|item| item.char_index);
    }

    let sentences: Vec<ChapterSentenceInfo> = sentence_rows
        .iter()
        .map(|row| {
            let sentence_id = str_field(row, "sentence_id").to_string();
            ChapterSentenceInfo {
                sentence_id: sentence_id.clone(),
                sentence_index: int_field(row, "sentence_index"),
                content: str_field(row, "content").to_string(),
                word_count: int_field(row, "word_count"),
                polyphones: polyphones_map.remove(&sentence_id).unwrap_or_default(),
            }
        })
        .collect();

    Ok(ChapterContentInfo {
        project_id: project.project_id,
        file_id: project.file_id,
        chapter_id: str_field(chapter_row, "chapter_id").to_string(),
        chapter_index: int_field(chapter_row, "chapter_index"),
        chapter_title: str_field(chapter_row, "chapter_title").to_string(),
        word_count: int_field(chapter_row, "word_count"),
        content: str_field(chapter_row, "content").to_string(),
        sentences,
    })
}

/// 修改章节标题与/或正文。正文变更时会重建切句，并清理该章原有的多音字标注。
pub fn update_chapter(
    store: &mut Store,
    project_id: &str,
    chapter_id: &str,
    chapter_title: Option<&str>,
    content: Option<&str>,
) -> AppResult<ChapterContentInfo> {
    let project = get_project(store, project_id, false)?;
    let chapter_row = store
        .contents
        .find_by("chapter_id", chapter_id)
        .filter(|row| str_field(row, "novel_id") == project.file_id)
        .cloned()
        .ok_or_else(|| AppError::file_not_found(format!("章节不存在：{chapter_id}")))?;

    let new_title = match chapter_title {
        Some(title) => {
            let title = title.trim();
            if title.is_empty() {
                return Err(AppError::invalid_parameter("章节标题不能为空"));
            }
            title.to_string()
        }
        None => str_field(&chapter_row, "chapter_title").to_string(),
    };

    let new_content = match content {
        Some(value) => {
            if value.trim().is_empty() {
                return Err(AppError::invalid_parameter("章节正文不能为空"));
            }
            Some(value.to_string())
        }
        None => None,
    };

    let content_changed = new_content
        .as_deref()
        .is_some_and(|value| value != str_field(&chapter_row, "content"));
    let word_count = match &new_content {
        Some(value) => count_words(value) as i64,
        None => int_field(&chapter_row, "word_count"),
    };

    store.contents.update_by(
        "chapter_id",
        chapter_id,
        &[
            ("chapter_title", new_title.clone()),
            (
                "content",
                new_content
                    .clone()
                    .unwrap_or_else(|| str_field(&chapter_row, "content").to_string()),
            ),
            ("word_count", word_count.to_string()),
        ],
    );

    if content_changed {
        let new_content = new_content.as_deref().unwrap_or_default();
        let sentence_ids: Vec<String> = store
            .sentences
            .find_all()
            .iter()
            .filter(|row| str_field(row, "chapter_id") == chapter_id)
            .map(|row| str_field(row, "sentence_id").to_string())
            .collect();
        for sentence_id in &sentence_ids {
            store.polyphones.delete_by("sentence_id", sentence_id);
        }
        store.sentences.delete_by("chapter_id", chapter_id);
        if !new_content.is_empty() {
            let records = create_sentence_records(&project.file_id, chapter_id, new_content);
            store.sentences.insert_batch(records);
        }
    }

    let all_chapters = text_service::get_novel_chapters(store, &project.file_id);
    let total_chapters = all_chapters.len() as i64;
    let total_words: i64 = all_chapters
        .iter()
        .map(|row| int_field(row, "word_count"))
        .sum();
    let now = Utc::now();

    store.novels.update_by(
        "novel_id",
        &project.file_id,
        &[
            ("chapter_count", total_chapters.to_string()),
            ("total_words", total_words.to_string()),
        ],
    );
    store.projects.update_by(
        "project_id",
        &project.project_id,
        &[
            ("chapter_count", total_chapters.to_string()),
            ("total_words", total_words.to_string()),
            ("updated_at", now.to_rfc3339()),
        ],
    );

    store.save_polyphones()?;
    store.save_sentences()?;
    store.save_contents()?;
    store.save_novels()?;
    store.save_projects()?;

    get_project_chapter_content(store, project_id, chapter_id)
}

/// 删除项目中的指定章节及其切句、多音字标注，并刷新项目统计。
pub fn delete_chapter(
    store: &mut Store,
    project_id: &str,
    chapter_id: &str,
) -> AppResult<ChapterDeleteInfo> {
    let project = get_project(store, project_id, false)?;
    let chapter_row = store
        .contents
        .find_by("chapter_id", chapter_id)
        .filter(|row| str_field(row, "novel_id") == project.file_id)
        .cloned()
        .ok_or_else(|| AppError::file_not_found(format!("章节不存在：{chapter_id}")))?;
    let chapter_title = str_field(&chapter_row, "chapter_title").to_string();

    let sentence_ids: Vec<String> = store
        .sentences
        .find_all()
        .iter()
        .filter(|row| str_field(row, "chapter_id") == chapter_id)
        .map(|row| str_field(row, "sentence_id").to_string())
        .collect();
    for sentence_id in &sentence_ids {
        store.polyphones.delete_by("sentence_id", sentence_id);
    }
    store.sentences.delete_by("chapter_id", chapter_id);
    store.contents.delete_by("chapter_id", chapter_id);

    let all_chapters = text_service::get_novel_chapters(store, &project.file_id);
    let chapter_count = all_chapters.len() as i64;
    let total_words: i64 = all_chapters
        .iter()
        .map(|row| int_field(row, "word_count"))
        .sum();
    let now = Utc::now();

    store.novels.update_by(
        "novel_id",
        &project.file_id,
        &[
            ("chapter_count", chapter_count.to_string()),
            ("total_words", total_words.to_string()),
        ],
    );
    store.projects.update_by(
        "project_id",
        &project.project_id,
        &[
            ("chapter_count", chapter_count.to_string()),
            ("total_words", total_words.to_string()),
            ("updated_at", now.to_rfc3339()),
        ],
    );

    store.save_polyphones()?;
    store.save_sentences()?;
    store.save_contents()?;
    store.save_novels()?;
    store.save_projects()?;

    Ok(ChapterDeleteInfo {
        project_id: project.project_id,
        chapter_id: chapter_id.to_string(),
        chapter_title,
        deleted: true,
        chapter_count,
        total_words,
        updated_at: now,
    })
}

/// 向已有项目安全追加小说章节。
#[allow(clippy::too_many_arguments)]
pub fn append_chapters(
    store: &mut Store,
    settings: &Settings,
    project_id: &str,
    filename: &str,
    content: &[u8],
    content_type: Option<&str>,
    overwrite_existing: bool,
    dry_run: bool,
) -> AppResult<ProjectChapterAppendInfo> {
    let _ = content_type;
    let project = get_project(store, project_id, false)?;

    let clean_filename = text_service::sanitize_filename(filename, settings.max_filename_length)?;
    let extension = text_service::extension_of(std::path::Path::new(&clean_filename));
    text_service::check_extension(&extension, &settings.allowed_extensions)?;

    if content.is_empty() {
        return Err(AppError::invalid_parameter("上传文件内容为空"));
    }
    if content.len() > settings.max_bytes() {
        return Err(AppError::file_too_large(format!(
            "文件大小超出限制（最大 {} MB）",
            settings.max_bytes() / (1024 * 1024)
        )));
    }

    let upload_file_id = uuid::Uuid::new_v4().simple().to_string();
    let target = store.path_for(&upload_file_id, &extension);
    std::fs::write(&target, content).map_err(|err| {
        let _ = std::fs::remove_file(&target);
        crate::error::storage_io("保存文件失败", err)
    })?;

    let content_text = text_service::decode_text(content);
    let cleaned = clean_text(&content_text);
    let mut new_parsed_chapters = parse_chapters(&cleaned);
    if new_parsed_chapters.is_empty() {
        new_parsed_chapters = vec![("全文".to_string(), cleaned.clone())];
    }

    let mut existing_chapter_rows = text_service::get_novel_chapters(store, &project.file_id);
    existing_chapter_rows.sort_by_key(|row| int_field(row, "chapter_index"));

    let mut existing_key_map: HashMap<String, Record> = HashMap::new();
    for row in &existing_chapter_rows {
        let key = resolve_chapter_key(str_field(row, "chapter_title"));
        existing_key_map.insert(key, row.clone());
    }

    let mut conflicts = Vec::new();
    for (chapter_title, chapter_content) in &new_parsed_chapters {
        let key = resolve_chapter_key(chapter_title);
        if let Some(old_row) = existing_key_map.get(&key) {
            let old_content = str_field(old_row, "content");
            let old_words = int_field(old_row, "word_count");
            let new_words = count_words(chapter_content) as i64;
            if old_content != chapter_content || old_words != new_words {
                conflicts.push(ChapterConflictInfo {
                    chapter_id: str_field(old_row, "chapter_id").to_string(),
                    chapter_index: int_field(old_row, "chapter_index"),
                    chapter_title: str_field(old_row, "chapter_title").to_string(),
                    existing_word_count: old_words,
                    incoming_word_count: new_words,
                    word_diff: new_words - old_words,
                });
            }
        }
    }

    let existing_total_words: i64 = existing_chapter_rows
        .iter()
        .map(|row| int_field(row, "word_count"))
        .sum();
    let mut simulated_added = 0i64;
    let mut simulated_skipped = 0i64;
    let mut simulated_chapters = Vec::new();
    let mut projected_total_words = existing_total_words;
    for (chapter_title, chapter_content) in &new_parsed_chapters {
        let key = resolve_chapter_key(chapter_title);
        let new_words = count_words(chapter_content) as i64;
        if let Some(old_row) = existing_key_map.get(&key) {
            simulated_skipped += 1;
            if overwrite_existing {
                projected_total_words += new_words - int_field(old_row, "word_count");
            }
            simulated_chapters.push(ChapterAppendItem {
                chapter_id: Some(str_field(old_row, "chapter_id").to_string()),
                chapter_index: Some(int_field(old_row, "chapter_index")),
                chapter_title: chapter_title.clone(),
                action: if overwrite_existing {
                    "overwritten".to_string()
                } else {
                    "skipped".to_string()
                },
                word_count: new_words,
            });
        } else {
            simulated_added += 1;
            projected_total_words += new_words;
            simulated_chapters.push(ChapterAppendItem {
                chapter_id: None,
                chapter_index: None,
                chapter_title: chapter_title.clone(),
                action: "added".to_string(),
                word_count: new_words,
            });
        }
    }

    if dry_run {
        let _ = std::fs::remove_file(&target);
        let projected_chapters = if !overwrite_existing {
            existing_chapter_rows.len() as i64 + simulated_added
        } else {
            existing_chapter_rows.len() as i64
                + (new_parsed_chapters.len() as i64 - simulated_skipped)
        };
        return Ok(ProjectChapterAppendInfo {
            project_id: project.project_id,
            file_id: project.file_id,
            dry_run: true,
            added_chapters: simulated_added,
            skipped_chapters: simulated_skipped,
            total_chapters: projected_chapters,
            total_words: projected_total_words,
            has_conflicts: !conflicts.is_empty(),
            conflicts,
            chapters: simulated_chapters,
            updated_at: project.updated_at,
        });
    }

    let mut added_chapters = 0i64;
    let mut skipped_chapters = 0i64;
    let mut new_chapter_records = Vec::new();
    let mut new_sentence_records = Vec::new();
    let mut processed_chapters = Vec::new();

    let numbered_positions: Vec<usize> = existing_chapter_rows
        .iter()
        .enumerate()
        .filter_map(|(index, row)| chapter_number(str_field(row, "chapter_title")).map(|_| index))
        .collect();
    if !numbered_positions.is_empty() {
        let mut numbered_rows: Vec<Record> = numbered_positions
            .iter()
            .map(|&index| existing_chapter_rows[index].clone())
            .collect();
        numbered_rows
            .sort_by_key(|row| chapter_number(str_field(row, "chapter_title")).unwrap_or(0));
        for (slot, row) in numbered_positions.into_iter().zip(numbered_rows) {
            existing_chapter_rows[slot] = row;
        }
    }

    for (chapter_title, chapter_content) in &new_parsed_chapters {
        let key = resolve_chapter_key(chapter_title);
        if existing_key_map.contains_key(&key) {
            if !overwrite_existing {
                let old_row = existing_key_map.get(&key).unwrap();
                processed_chapters.push(ChapterAppendItem {
                    chapter_id: Some(str_field(old_row, "chapter_id").to_string()),
                    chapter_index: Some(int_field(old_row, "chapter_index")),
                    chapter_title: chapter_title.clone(),
                    action: "skipped".to_string(),
                    word_count: int_field(old_row, "word_count"),
                });
                skipped_chapters += 1;
                continue;
            }

            let old_row = existing_key_map.get(&key).unwrap();
            let old_chapter_id = str_field(old_row, "chapter_id").to_string();
            let word_count = count_words(chapter_content);

            store.contents.update_by(
                "chapter_id",
                &old_chapter_id,
                &[
                    ("chapter_title", chapter_title.clone()),
                    ("content", chapter_content.clone()),
                    ("word_count", word_count.to_string()),
                ],
            );
            if let Some(row) = existing_chapter_rows
                .iter_mut()
                .find(|row| str_field(row, "chapter_id") == old_chapter_id)
            {
                row.insert("chapter_title".into(), chapter_title.clone());
                row.insert("content".into(), chapter_content.clone());
                row.insert("word_count".into(), word_count.to_string());
            }

            let old_sentence_rows: Vec<Record> = store
                .sentences
                .find_all()
                .iter()
                .filter(|row| str_field(row, "chapter_id") == old_chapter_id)
                .cloned()
                .collect();
            for row in &old_sentence_rows {
                store
                    .polyphones
                    .delete_by("sentence_id", str_field(row, "sentence_id"));
            }
            store.sentences.delete_by("chapter_id", &old_chapter_id);
            if !chapter_content.is_empty() {
                let overwritten =
                    create_sentence_records(&project.file_id, &old_chapter_id, chapter_content);
                store.sentences.insert_batch(overwritten);
            }
            skipped_chapters += 1;
            processed_chapters.push(ChapterAppendItem {
                chapter_id: Some(old_chapter_id),
                chapter_index: None,
                chapter_title: chapter_title.clone(),
                action: "overwritten".to_string(),
                word_count: word_count as i64,
            });
            continue;
        }

        added_chapters += 1;
        let chapter_id = uuid::Uuid::new_v4().simple().to_string();
        let word_count = count_words(chapter_content);
        processed_chapters.push(ChapterAppendItem {
            chapter_id: Some(chapter_id.clone()),
            chapter_index: None,
            chapter_title: chapter_title.clone(),
            action: "added".to_string(),
            word_count: word_count as i64,
        });

        let mut chapter_record = Record::new();
        chapter_record.insert("chapter_id".into(), chapter_id.clone());
        chapter_record.insert("novel_id".into(), project.file_id.clone());
        chapter_record.insert("chapter_index".into(), "0".to_string());
        chapter_record.insert("chapter_title".into(), chapter_title.clone());
        chapter_record.insert("content".into(), chapter_content.clone());
        chapter_record.insert("word_count".into(), word_count.to_string());

        let insert_at = chapter_number(chapter_title)
            .and_then(|number| {
                existing_chapter_rows.iter().position(|row| {
                    chapter_number(str_field(row, "chapter_title"))
                        .map(|existing| existing > number)
                        .unwrap_or(false)
                })
            })
            .unwrap_or(existing_chapter_rows.len());
        existing_chapter_rows.insert(insert_at, chapter_record.clone());
        new_chapter_records.push(chapter_record);

        if !chapter_content.is_empty() {
            new_sentence_records.extend(create_sentence_records(
                &project.file_id,
                &chapter_id,
                chapter_content,
            ));
        }
    }

    store.contents.insert_batch(new_chapter_records);

    for (index, row) in existing_chapter_rows.iter_mut().enumerate() {
        let chapter_id = str_field(row, "chapter_id").to_string();
        let index_value = index.to_string();
        row.insert("chapter_index".into(), index_value.clone());
        store
            .contents
            .update_by("chapter_id", &chapter_id, &[("chapter_index", index_value)]);
    }

    let chapter_index_map: HashMap<String, i64> = existing_chapter_rows
        .iter()
        .map(|row| {
            (
                str_field(row, "chapter_id").to_string(),
                int_field(row, "chapter_index"),
            )
        })
        .collect();
    for item in &mut processed_chapters {
        if let Some(chapter_id) = &item.chapter_id {
            item.chapter_index = chapter_index_map.get(chapter_id).copied();
        }
    }

    store.sentences.insert_batch(new_sentence_records);
    store.save_contents()?;
    store.save_sentences()?;

    let all_chapters = text_service::get_novel_chapters(store, &project.file_id);
    let total_chapters = all_chapters.len() as i64;
    let total_words: i64 = all_chapters
        .iter()
        .map(|row| int_field(row, "word_count"))
        .sum();

    let now = Utc::now();
    store.novels.update_by(
        "novel_id",
        &project.file_id,
        &[
            ("chapter_count", total_chapters.to_string()),
            ("total_words", total_words.to_string()),
        ],
    );
    store.projects.update_by(
        "project_id",
        &project.project_id,
        &[
            ("filename", clean_filename),
            ("chapter_count", total_chapters.to_string()),
            ("total_words", total_words.to_string()),
            ("updated_at", now.to_rfc3339()),
        ],
    );
    store.save_novels()?;
    store.save_projects()?;

    let _ = std::fs::remove_file(&target);

    Ok(ProjectChapterAppendInfo {
        project_id: project.project_id,
        file_id: project.file_id,
        dry_run: false,
        added_chapters,
        skipped_chapters,
        total_chapters,
        total_words,
        has_conflicts: !conflicts.is_empty(),
        conflicts,
        chapters: processed_chapters,
        updated_at: now,
    })
}

/// 获取指定章节内所有句子的多音字标注。
pub fn get_chapter_polyphones(
    store: &Store,
    project_id: &str,
    chapter_id: &str,
) -> AppResult<ChapterPolyphoneList> {
    let project = get_project(store, project_id, false)?;
    let chapter_row = store
        .contents
        .find_by("chapter_id", chapter_id)
        .filter(|row| str_field(row, "novel_id") == project.file_id)
        .ok_or_else(|| AppError::file_not_found(format!("章节不存在：{chapter_id}")))?;
    let chapter_title = str_field(chapter_row, "chapter_title").to_string();

    let mut sentence_rows: Vec<Record> = store
        .sentences
        .find_all()
        .iter()
        .filter(|row| {
            str_field(row, "chapter_id") == chapter_id
                && str_field(row, "novel_id") == project.file_id
        })
        .cloned()
        .collect();
    sentence_rows.sort_by_key(|row| int_field(row, "sentence_index"));

    let mut items = Vec::new();
    let mut total = 0usize;
    for sentence in sentence_rows {
        let sentence_id = str_field(&sentence, "sentence_id").to_string();
        let mut polyphones: Vec<SentencePolyphoneInfo> = store
            .polyphones
            .find_all()
            .iter()
            .filter(|row| str_field(row, "sentence_id") == sentence_id)
            .map(record_to_polyphone_info)
            .collect();
        if polyphones.is_empty() {
            continue;
        }
        polyphones.sort_by_key(|item| item.char_index);
        total += polyphones.len();
        items.push(ChapterPolyphoneSentence {
            sentence_id,
            sentence_index: int_field(&sentence, "sentence_index"),
            content: str_field(&sentence, "content").to_string(),
            polyphones,
        });
    }

    Ok(ChapterPolyphoneList {
        chapter_id: chapter_id.to_string(),
        chapter_title,
        sentence_count: items.len(),
        total,
        items,
    })
}

fn ensure_chapter_sentence(
    store: &Store,
    project_id: &str,
    chapter_id: &str,
    sentence_id: &str,
) -> AppResult<()> {
    let project = get_project(store, project_id, false)?;
    store
        .contents
        .find_by("chapter_id", chapter_id)
        .filter(|row| str_field(row, "novel_id") == project.file_id)
        .ok_or_else(|| AppError::file_not_found(format!("章节不存在：{chapter_id}")))?;
    store
        .sentences
        .find_by("sentence_id", sentence_id)
        .filter(|row| {
            str_field(row, "novel_id") == project.file_id
                && str_field(row, "chapter_id") == chapter_id
        })
        .ok_or_else(|| {
            AppError::file_not_found(format!("句子不存在或不属于该章节：{sentence_id}"))
        })?;
    Ok(())
}

/// 获取章节下指定句子的多音字标注。
pub fn get_chapter_sentence_polyphones(
    store: &Store,
    project_id: &str,
    chapter_id: &str,
    sentence_id: &str,
) -> AppResult<Vec<SentencePolyphoneInfo>> {
    ensure_chapter_sentence(store, project_id, chapter_id, sentence_id)?;
    get_sentence_polyphones(store, project_id, sentence_id)
}

/// 保存章节下指定句子的多音字标注。
pub fn save_chapter_sentence_polyphones(
    store: &mut Store,
    project_id: &str,
    chapter_id: &str,
    sentence_id: &str,
    annotations: &[super::schema::PolyphoneAnnotation],
) -> AppResult<Vec<SentencePolyphoneInfo>> {
    ensure_chapter_sentence(store, project_id, chapter_id, sentence_id)?;
    save_sentence_polyphones(store, project_id, sentence_id, annotations)
}

/// 清空章节下指定句子的多音字标注。
pub fn delete_chapter_sentence_polyphones(
    store: &mut Store,
    project_id: &str,
    chapter_id: &str,
    sentence_id: &str,
) -> AppResult<usize> {
    ensure_chapter_sentence(store, project_id, chapter_id, sentence_id)?;
    delete_sentence_polyphones(store, project_id, sentence_id)
}

/// 删除章节下指定句子中的单条多音字标注。
pub fn delete_chapter_sentence_polyphone(
    store: &mut Store,
    project_id: &str,
    chapter_id: &str,
    sentence_id: &str,
    char_index: i64,
) -> AppResult<SentencePolyphoneItemDeleteInfo> {
    ensure_chapter_sentence(store, project_id, chapter_id, sentence_id)?;

    let record = store
        .polyphones
        .find_all()
        .iter()
        .find(|row| {
            str_field(row, "sentence_id") == sentence_id
                && int_field(row, "char_index") == char_index
        })
        .cloned()
        .ok_or_else(|| {
            AppError::file_not_found(format!(
                "多音字标注不存在：sentence_id={sentence_id}, char_index={char_index}"
            ))
        })?;

    let polyphone_id = str_field(&record, "polyphone_id").to_string();
    store.polyphones.delete_by("polyphone_id", &polyphone_id);
    store.save_polyphones()?;

    Ok(SentencePolyphoneItemDeleteInfo {
        polyphone_id,
        sentence_id: sentence_id.to_string(),
        char_index,
        deleted: true,
    })
}

/// 获取指定句子的所有多音字标注。
pub fn get_sentence_polyphones(
    store: &Store,
    project_id: &str,
    sentence_id: &str,
) -> AppResult<Vec<SentencePolyphoneInfo>> {
    let project = get_project(store, project_id, false)?;
    let sentence_row = store
        .sentences
        .find_by("sentence_id", sentence_id)
        .filter(|row| str_field(row, "novel_id") == project.file_id)
        .ok_or_else(|| AppError::file_not_found(format!("句子不存在：{sentence_id}")))?;

    let mut items: Vec<SentencePolyphoneInfo> = store
        .polyphones
        .find_all()
        .iter()
        .filter(|row| str_field(row, "sentence_id") == sentence_id)
        .map(record_to_polyphone_info)
        .collect();
    items.sort_by_key(|item| item.char_index);
    let _ = sentence_row;
    Ok(items)
}

/// 全量覆盖保存句子的多音字标注。
pub fn save_sentence_polyphones(
    store: &mut Store,
    project_id: &str,
    sentence_id: &str,
    annotations: &[super::schema::PolyphoneAnnotation],
) -> AppResult<Vec<SentencePolyphoneInfo>> {
    let project = get_project(store, project_id, false)?;
    let sentence_row = store
        .sentences
        .find_by("sentence_id", sentence_id)
        .filter(|row| str_field(row, "novel_id") == project.file_id)
        .ok_or_else(|| AppError::file_not_found(format!("句子不存在：{sentence_id}")))?
        .clone();

    let sentence_content = str_field(&sentence_row, "content").to_string();
    let sentence_chars: Vec<char> = sentence_content.chars().collect();
    let mut seen_indices = std::collections::HashSet::new();
    let now = Utc::now();
    let now_iso = now.to_rfc3339();
    let mut records = Vec::new();

    for annotation in annotations {
        let index = annotation.char_index;
        let character = annotation.character.trim();
        let raw_pinyin = annotation.pinyin.trim();

        if raw_pinyin.is_empty() {
            return Err(AppError::invalid_parameter("拼音不能为空"));
        }
        if character.is_empty() || character.chars().count() != 1 {
            return Err(AppError::invalid_parameter("标注字符不能为空"));
        }

        let (derived_base, derived_tone, derived_tts) =
            crate::text::parsers::parse_pinyin_elements(raw_pinyin);
        let pinyin_base = annotation
            .pinyin_base
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| value.to_string())
            .unwrap_or(derived_base);

        let tone = match annotation.tone {
            Some(value) => value,
            None => derived_tone as i64,
        };

        let tts_pinyin = annotation
            .tts_pinyin
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| value.to_string())
            .unwrap_or_else(|| {
                if !pinyin_base.is_empty() {
                    format!("{pinyin_base}{tone}")
                } else {
                    derived_tts.clone()
                }
            });

        if index < 0 || index as usize >= sentence_chars.len() {
            return Err(AppError::invalid_parameter(format!(
                "字符偏移量 char_index={index} 超出句子长度范围 (0~{})",
                sentence_chars.len().saturating_sub(1)
            )));
        }
        if !seen_indices.insert(index) {
            return Err(AppError::invalid_parameter(format!(
                "字符偏移量 char_index={index} 重复"
            )));
        }

        let actual_char = sentence_chars[index as usize];
        if actual_char.to_string() != character {
            return Err(AppError::invalid_parameter(format!(
                "char_index={index} 位置的实际字符为 '{actual_char}'，与标注字符 '{character}' 不匹配"
            )));
        }

        let mut record = Record::new();
        record.insert(
            "polyphone_id".into(),
            uuid::Uuid::new_v4().simple().to_string(),
        );
        record.insert("sentence_id".into(), sentence_id.to_string());
        record.insert("char_index".into(), index.to_string());
        record.insert("character".into(), character.to_string());
        record.insert("pinyin".into(), raw_pinyin.to_string());
        record.insert("pinyin_base".into(), pinyin_base);
        record.insert("tone".into(), tone.to_string());
        record.insert("tts_pinyin".into(), tts_pinyin);
        record.insert("created_at".into(), now_iso.clone());
        record.insert("updated_at".into(), now_iso.clone());
        records.push(record);
    }

    store.polyphones.delete_by("sentence_id", sentence_id);
    store.polyphones.insert_batch(records.clone());
    store.save_polyphones()?;

    let mut result: Vec<SentencePolyphoneInfo> =
        records.iter().map(record_to_polyphone_info).collect();
    result.sort_by_key(|item| item.char_index);
    Ok(result)
}

/// 删除指定句子的所有多音字标注，返回被删除的数量。
pub fn delete_sentence_polyphones(
    store: &mut Store,
    project_id: &str,
    sentence_id: &str,
) -> AppResult<usize> {
    let project = get_project(store, project_id, false)?;
    store
        .sentences
        .find_by("sentence_id", sentence_id)
        .filter(|row| str_field(row, "novel_id") == project.file_id)
        .ok_or_else(|| AppError::file_not_found(format!("句子不存在：{sentence_id}")))?;

    let deleted_count = store
        .polyphones
        .find_all()
        .iter()
        .filter(|row| str_field(row, "sentence_id") == sentence_id)
        .count();
    if deleted_count > 0 {
        store.polyphones.delete_by("sentence_id", sentence_id);
        store.save_polyphones()?;
    }
    Ok(deleted_count)
}

/// 按条件查询项目列表（v2）。
#[allow(clippy::too_many_arguments)]
pub fn query_projects(
    store: &Store,
    page: usize,
    page_size: usize,
    in_recycle_bin: bool,
    status: Option<&str>,
    keyword: Option<&str>,
    sort: &str,
    order: &str,
) -> (Vec<ProjectInfo>, usize) {
    let mut items: Vec<ProjectInfo> = store
        .projects
        .find_all()
        .iter()
        .map(record_to_info)
        .collect();

    items.retain(|item| item.is_deleted == in_recycle_bin);

    if let Some(status) = status.map(str::trim).filter(|value| !value.is_empty()) {
        items.retain(|item| item.status == status);
    }
    if let Some(keyword) = keyword.map(str::trim).filter(|value| !value.is_empty()) {
        items.retain(|item| item.name.contains(keyword));
    }

    let ascending = order.eq_ignore_ascii_case("asc");
    match sort {
        "updated_at" => items.sort_by(|a, b| {
            if ascending {
                a.updated_at.cmp(&b.updated_at)
            } else {
                b.updated_at.cmp(&a.updated_at)
            }
        }),
        "name" => items.sort_by(|a, b| {
            if ascending {
                a.name.cmp(&b.name)
            } else {
                b.name.cmp(&a.name)
            }
        }),
        _ => items.sort_by(|a, b| {
            if ascending {
                a.created_at.cmp(&b.created_at)
            } else {
                b.created_at.cmp(&a.created_at)
            }
        }),
    }

    let total = items.len();
    let start = page.saturating_sub(1).saturating_mul(page_size);
    let items = items.into_iter().skip(start).take(page_size).collect();
    (items, total)
}

/// 更新项目名称与/或制作状态（v2）。
pub fn update_project_fields(
    store: &mut Store,
    project_id: &str,
    name: Option<&str>,
    status: Option<&str>,
) -> AppResult<ProjectInfo> {
    let mut updates: Vec<(&str, String)> = Vec::new();

    if let Some(name) = name {
        let actual_name = name.trim();
        if actual_name.is_empty() {
            return Err(AppError::invalid_parameter("项目名称不能为空"));
        }
        if actual_name.chars().count() > 255 {
            return Err(AppError::invalid_parameter(
                "项目名称长度不能超过 255 个字符",
            ));
        }
        updates.push(("name", actual_name.to_string()));
    }

    if let Some(status) = status {
        let actual_status = status.trim();
        if actual_status.is_empty() {
            return Err(AppError::invalid_parameter("制作状态不能为空"));
        }
        updates.push(("status", actual_status.to_string()));
    }

    if updates.is_empty() {
        return Err(AppError::invalid_parameter("请提供要更新的字段"));
    }

    get_project(store, project_id, false)?;
    let now = Utc::now();
    updates.push(("updated_at", now.to_rfc3339()));
    store.projects.update_by("project_id", project_id, &updates);
    store.save_projects()?;
    get_project(store, project_id, false)
}

/// 兼容旧版辅助：解析时间字符串。
pub fn parse_time(value: &str) -> DateTime<Utc> {
    text_service::parse_datetime(value)
}

/// 在项目内按关键词搜索章节（标题或正文命中），返回章节元信息与摘要。
pub fn search_chapters(
    store: &Store,
    project_id: &str,
    keyword: &str,
    limit: usize,
) -> AppResult<Vec<ChapterSearchHit>> {
    let project = get_project(store, project_id, false)?;
    let needle = keyword.trim();
    if needle.is_empty() || limit == 0 {
        return Ok(Vec::new());
    }

    let mut items: Vec<ChapterSearchHit> = store
        .contents
        .find_all()
        .iter()
        .filter(|row| str_field(row, "novel_id") == project.file_id)
        .filter_map(|row| {
            let title = str_field(row, "chapter_title");
            let content = str_field(row, "content");
            let position = title
                .find(needle)
                .map(|index| (true, index))
                .or_else(|| content.find(needle).map(|index| (false, index)))?;
            let (in_title, byte_index) = position;
            let source = if in_title { title } else { content };
            let chars: Vec<char> = source.chars().collect();
            let start = source[..byte_index].chars().count().min(chars.len());
            let from = start.saturating_sub(40);
            let to = (start + needle.chars().count() + 40).min(chars.len());
            let excerpt: String = chars[from..to].iter().collect();
            Some(ChapterSearchHit {
                chapter_id: str_field(row, "chapter_id").to_string(),
                chapter_index: int_field(row, "chapter_index"),
                chapter_title: title.to_string(),
                word_count: int_field(row, "word_count"),
                excerpt,
            })
        })
        .collect();

    items.sort_by_key(|item| item.chapter_index);
    items.truncate(limit);
    Ok(items)
}

/// 全库统计：剧目数、章节数、总字数与回收站数量。
pub fn get_stats(store: &Store) -> ProjectStats {
    let mut stats = ProjectStats {
        projects: 0,
        chapters: 0,
        words: 0,
        recycle_bin: 0,
    };

    for record in store.projects.find_all() {
        if bool_field(record, "is_deleted") {
            stats.recycle_bin += 1;
        } else {
            stats.projects += 1;
            stats.chapters += int_field(record, "chapter_count");
            stats.words += int_field(record, "total_words");
        }
    }

    stats
}
