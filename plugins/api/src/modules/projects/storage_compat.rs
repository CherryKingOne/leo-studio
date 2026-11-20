use std::path::Path;
use chrono::{DateTime, TimeZone, Utc};

use crate::config::Settings;
use crate::error::{AppError, AppResult, storage_io};
use crate::storage::csv_table::{Record, str_field};
use crate::storage::store::Store;
use crate::text::{
    TextPipeline, clean_dialogue_records, count_words, create_dialogue_records_from_sentence,
    create_sentence_records, parse_novel_filename,
};

pub fn parse_datetime(value: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.with_timezone(&Utc))
        .or_else(|_| {
            chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S")
                .map(|naive| Utc.from_utc_datetime(&naive))
        })
        .unwrap_or_else(|_| Utc::now())
}

pub fn sanitize_filename(filename: &str, max_length: usize) -> AppResult<String> {
    if filename.trim().is_empty() {
        return Err(AppError::invalid_parameter("文件名不能为空"));
    }

    let normalized = filename.replace('\\', "/");
    let last = normalized.split('/').next_back().unwrap_or("");
    let sanitized: String = last
        .chars()
        .map(|ch| {
            let allowed = ch.is_alphanumeric()
                || ch == '_'
                || ch == '-'
                || ch == '.'
                || ch == '—'
                || ch == '·'
                || ('\u{4e00}'..='\u{9fff}').contains(&ch);
            if allowed { ch } else { '_' }
        })
        .collect();
    let name = sanitized.trim_matches(['.', '_', ' ']);
    if name.is_empty() {
        return Err(AppError::invalid_parameter("文件名不合法"));
    }
    if name.chars().count() > max_length {
        return Err(AppError::invalid_parameter(format!(
            "文件名长度不能超过 {max_length} 个字符"
        )));
    }
    Ok(name.to_string())
}

pub fn check_extension(extension: &str, allowed: &[String]) -> AppResult<()> {
    if allowed.is_empty() {
        return Ok(());
    }
    let normalized: Vec<String> = allowed.iter().map(|item| item.to_lowercase()).collect();
    if !normalized.contains(&extension.to_lowercase()) {
        return Err(AppError::unsupported_file_type(format!(
            "不支持的文件类型 {extension}，允许的类型：{}",
            normalized.join(", ")
        )));
    }
    Ok(())
}

pub fn extension_of(path: &Path) -> String {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| format!(".{}", ext.to_lowercase()))
        .unwrap_or_default()
}

pub fn decode_text(content: &[u8]) -> String {
    if let Ok(utf8) = std::str::from_utf8(content) {
        return utf8.to_string();
    }
    let (cow, _, had_errors) = encoding_rs::GBK.decode(content);
    if !had_errors {
        return cow.into_owned();
    }
    String::from_utf8_lossy(content).into_owned()
}

pub fn get_novel_chapters(store: &Store, file_id: &str) -> Vec<Record> {
    store
        .contents
        .find_all()
        .iter()
        .filter(|row| str_field(row, "novel_id") == file_id)
        .cloned()
        .collect()
}

pub fn get_chapter_sentences(store: &Store, chapter_id: &str) -> Vec<Record> {
    store
        .sentences
        .find_all()
        .iter()
        .filter(|row| str_field(row, "chapter_id") == chapter_id)
        .cloned()
        .collect()
}

pub struct SavedFileInfo {
    pub file_id: String,
    pub filename: String,
    pub size: usize,
}

pub fn save_file(
    store: &mut Store,
    settings: &Settings,
    text: &TextPipeline,
    filename: &str,
    content: &[u8],
    _content_type: Option<&str>,
    _description: Option<&str>,
) -> AppResult<SavedFileInfo> {
    let (parsed_title, _, _) = parse_novel_filename(filename);
    let safe_name = sanitize_filename(filename, settings.max_filename_length)?;
    let extension = extension_of(Path::new(&safe_name));
    check_extension(&extension, &settings.allowed_extensions)?;

    if content.len() > settings.max_bytes() {
        return Err(AppError::file_too_large("文件大小超过限制"));
    }
    if content.is_empty() {
        return Err(AppError::invalid_parameter("上传文件内容为空"));
    }

    let title = if parsed_title.is_empty() {
        Path::new(&safe_name)
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("")
            .to_string()
    } else {
        parsed_title
    };
    let file_format = extension.trim_start_matches('.').to_string();
    let file_id = uuid::Uuid::new_v4().simple().to_string();
    let target = store.path_for(&file_id, &extension);

    std::fs::write(&target, content).map_err(|err| {
        let _ = std::fs::remove_file(&target);
        storage_io("保存文件失败", err)
    })?;

    let content_text = decode_text(content);
    let cleaned = text.clean_text(&content_text);
    let mut chapters = text.parse_chapters(&cleaned);
    if chapters.is_empty() {
        chapters = vec![("全文".to_string(), cleaned.clone())];
    }

    let chapter_count = chapters.len();
    let total_words = count_words(&content_text);
    let line_count = content_text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .count();
    let now = Utc::now();

    let mut novel = Record::new();
    novel.insert("novel_id".into(), file_id.clone());
    novel.insert("title".into(), title);
    novel.insert("format".into(), file_format);
    novel.insert("chapter_count".into(), chapter_count.to_string());
    novel.insert("total_words".into(), total_words.to_string());
    novel.insert("line_count".into(), line_count.to_string());
    novel.insert("created_at".into(), now.to_rfc3339());
    store.novels.insert(novel);

    let mut chapter_records = Vec::new();
    let mut sentence_records = Vec::new();
    for (index, (chapter_title, chapter_content)) in chapters.iter().enumerate() {
        let chapter_id = uuid::Uuid::new_v4().simple().to_string();
        let mut record = Record::new();
        record.insert("chapter_id".into(), chapter_id.clone());
        record.insert("novel_id".into(), file_id.clone());
        record.insert("chapter_index".into(), index.to_string());
        record.insert("chapter_title".into(), chapter_title.clone());
        record.insert("content".into(), chapter_content.clone());
        record.insert(
            "word_count".into(),
            count_words(chapter_content).to_string(),
        );
        chapter_records.push(record);

        if !chapter_content.is_empty() {
            sentence_records.extend(create_sentence_records(
                text,
                &file_id,
                &chapter_id,
                chapter_content,
            ));
        }
    }

    store.contents.insert_batch(chapter_records);
    store.sentences.insert_batch(sentence_records);

    let dialogue_records: Vec<Record> = store
        .sentences
        .find_all()
        .iter()
        .filter(|row| str_field(row, "novel_id") == file_id)
        .flat_map(|row| create_dialogue_records_from_sentence(text, row))
        .collect();
    let dialogue_records = clean_dialogue_records(text, dialogue_records);
    store.dialogues.insert_batch(dialogue_records);

    store.save_novels()?;
    store.save_contents()?;
    store.save_sentences()?;
    store.save_dialogues()?;

    Ok(SavedFileInfo {
        file_id,
        filename: safe_name,
        size: content.len(),
    })
}

pub fn delete_file(store: &mut Store, file_id: &str) -> AppResult<()> {
    let target = store.path_for(file_id, ".txt");
    let _ = std::fs::remove_file(target);

    store.novels.delete_by("novel_id", file_id);
    store.contents.delete_by("novel_id", file_id);
    store.sentences.delete_by("novel_id", file_id);
    store.dialogues.delete_by("novel_id", file_id);
    store.save_novels()?;
    store.save_contents()?;
    store.save_sentences()?;
    store.save_dialogues()?;
    Ok(())
}

pub fn backfill_dialogues(store: &mut Store, text: &TextPipeline) -> AppResult<usize> {
    if !store.dialogues.find_all().is_empty() {
        return Ok(0);
    }
    let sentences = store.sentences.find_all().to_vec();
    if sentences.is_empty() {
        return Ok(0);
    }
    let dialogue_records: Vec<Record> = sentences
        .iter()
        .flat_map(|row| create_dialogue_records_from_sentence(text, row))
        .collect();
    let dialogue_records = clean_dialogue_records(text, dialogue_records);
    let count = dialogue_records.len();
    store.dialogues.insert_batch(dialogue_records);
    store.save_dialogues()?;
    Ok(count)
}
