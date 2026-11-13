use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use encoding_rs::{GB18030, GBK};

use crate::config::Settings;
use crate::error::{AppError, AppResult, storage_io};
use crate::storage::csv_table::{Record, bool_field, int_field, str_field};
use crate::storage::store::{Store, human_size};
use crate::text::parsers::{clean_text, count_words, parse_chapters, parse_novel_filename};
use crate::text::processor::create_sentence_records;

use super::model::{TextFileInfo, categorize, default_mime};

/// 清洗上传文件名，移除路径分隔符与危险字符。
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

/// 校验扩展名是否在文本文件白名单内。
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

/// 确定文件的 MIME 类型。
pub fn resolve_content_type(extension: &str, content_type: Option<&str>) -> String {
    if let Some(mime) = default_mime(extension) {
        return mime.to_string();
    }
    if let Some(value) = content_type {
        let value = value.trim();
        if !value.is_empty() {
            return value.to_string();
        }
    }
    "application/octet-stream".to_string()
}

/// 判断该扩展名的文件是否支持文本内容预览。
pub fn is_previewable(_extension: &str) -> bool {
    true
}

pub fn extension_of(path: &Path) -> String {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| format!(".{}", ext.to_lowercase()))
        .unwrap_or_default()
}

/// 保存单个文本文件并解析章节写入 CSV 表。
pub fn save_file(
    store: &mut Store,
    settings: &Settings,
    filename: &str,
    content: &[u8],
    content_type: Option<&str>,
    description: Option<&str>,
) -> AppResult<TextFileInfo> {
    let (parsed_title, _, _) = parse_novel_filename(filename);
    let safe_name = sanitize_filename(filename, settings.max_filename_length)?;
    let extension = extension_of(Path::new(&safe_name));
    check_extension(&extension, &settings.allowed_extensions)?;

    if content.len() > settings.max_bytes() {
        return Err(AppError::file_too_large(format!(
            "文件大小超过限制，最大允许 {}",
            human_size(settings.max_bytes())
        )));
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

    fs::write(&target, content).map_err(|err| {
        let _ = fs::remove_file(&target);
        storage_io("保存文件失败", err)
    })?;

    let content_text = decode_text(content);
    let cleaned = clean_text(&content_text);
    let mut chapters = parse_chapters(&cleaned);
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
                &file_id,
                &chapter_id,
                chapter_content,
            ));
        }
    }

    store.contents.insert_batch(chapter_records);
    store.sentences.insert_batch(sentence_records);
    store.save_novels()?;
    store.save_contents()?;
    store.save_sentences()?;

    let download_url = format!("/api/v1/text-files/{file_id}/download");
    Ok(TextFileInfo {
        file_id,
        filename: safe_name,
        stored_name: target
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("")
            .to_string(),
        extension,
        size: content.len(),
        content_type: resolve_content_type(&extension_of(&target), content_type),
        category: categorize(&extension_of(&target)),
        uploaded_at: now,
        download_url,
        description: description.map(|value| value.to_string()),
    })
}

pub fn decode_text(content: &[u8]) -> String {
    let text = match std::str::from_utf8(content) {
        Ok(text) => text.to_string(),
        Err(_) => {
            let (text, _, _) = GB18030.decode(content);
            text.into_owned()
        }
    };
    normalize_newlines(&text)
}

/// 与 Python `read_text` 的通用换行转换保持一致。
pub fn normalize_newlines(text: &str) -> String {
    if !text.contains('\r') {
        return text.to_string();
    }
    text.replace("\r\n", "\n").replace('\r', "\n")
}

/// 将小说记录转换为领域模型。
pub fn record_to_info(store: &Store, record: &Record) -> TextFileInfo {
    let novel_id = str_field(record, "novel_id").to_string();
    let title = str_field(record, "title").to_string();
    let format = {
        let value = str_field(record, "format");
        if value.is_empty() { "txt" } else { value }
    };
    let mut extension = format!(".{format}");

    let mut stored_name = format!("{novel_id}{extension}");
    let mut size = int_field(record, "total_words").max(0) as usize;

    if let Some(path) = find_stored_file(store, &novel_id) {
        stored_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("")
            .to_string();
        extension = extension_of(&path);
        if let Ok(metadata) = fs::metadata(&path) {
            size = metadata.len() as usize;
        }
    }

    let filename = if title.is_empty() {
        stored_name.clone()
    } else {
        format!("{title}{extension}")
    };

    let created_at_raw = {
        let value = str_field(record, "created_at");
        if value.is_empty() {
            str_field(record, "uploaded_at")
        } else {
            value
        }
    };
    let uploaded_at = parse_datetime(created_at_raw);

    let download_url = format!("/api/v1/text-files/{novel_id}/download");
    let content_type =
        resolve_content_type(&extension_of(Path::new(&stored_name)), Some("text/plain"));
    let category = categorize(&extension);

    TextFileInfo {
        file_id: novel_id,
        filename,
        stored_name,
        extension,
        size,
        content_type,
        category,
        uploaded_at,
        download_url,
        description: None,
    }
}

fn find_stored_file(store: &Store, novel_id: &str) -> Option<PathBuf> {
    let entries = fs::read_dir(&store.upload_dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.file_stem().and_then(|stem| stem.to_str()) == Some(novel_id) {
            return Some(path);
        }
    }
    None
}

pub fn parse_datetime(value: &str) -> DateTime<Utc> {
    if value.is_empty() {
        return Utc::now();
    }
    if let Ok(parsed) = DateTime::parse_from_rfc3339(value) {
        return parsed.with_timezone(&Utc);
    }
    if let Ok(parsed) = value.parse::<DateTime<Utc>>() {
        return parsed;
    }
    Utc::now()
}

pub fn get_file(store: &Store, file_id: &str) -> AppResult<TextFileInfo> {
    let record = store
        .novels
        .find_by("novel_id", file_id)
        .ok_or_else(|| AppError::file_not_found(format!("文件不存在：{file_id}")))?;
    Ok(record_to_info(store, record))
}

pub fn list_files(store: &Store, page: usize, page_size: usize) -> (Vec<TextFileInfo>, usize) {
    let mut items: Vec<TextFileInfo> = store
        .novels
        .find_all()
        .iter()
        .map(|record| record_to_info(store, record))
        .collect();
    items.sort_by_key(|item| std::cmp::Reverse(item.uploaded_at));

    let total = items.len();
    let start = page.saturating_sub(1).saturating_mul(page_size);
    let items = items.into_iter().skip(start).take(page_size).collect();
    (items, total)
}

pub fn delete_file(store: &mut Store, file_id: &str) -> AppResult<TextFileInfo> {
    let info = get_file(store, file_id)?;
    let target = store.upload_dir.join(&info.stored_name);
    if let Err(err) = fs::remove_file(&target)
        && err.kind() != std::io::ErrorKind::NotFound
    {
        return Err(storage_io("删除文件失败", err));
    }

    store.novels.delete_by("novel_id", file_id);
    store.contents.delete_by("novel_id", file_id);

    let sentence_ids: Vec<String> = store
        .sentences
        .find_all()
        .iter()
        .filter(|row| str_field(row, "novel_id") == file_id)
        .map(|row| str_field(row, "sentence_id").to_string())
        .collect();
    for sentence_id in sentence_ids {
        store.polyphones.delete_by("sentence_id", &sentence_id);
    }
    store.sentences.delete_by("novel_id", file_id);

    store.save_novels()?;
    store.save_contents()?;
    store.save_sentences()?;
    store.save_polyphones()?;
    Ok(info)
}

/// 打开文件用于下载，返回 (路径, 元数据)。
pub fn open_file(store: &Store, file_id: &str) -> AppResult<(PathBuf, TextFileInfo)> {
    let info = get_file(store, file_id)?;
    let path = store.upload_dir.join(&info.stored_name);
    if !path.exists() {
        return Err(AppError::file_not_found(format!("文件不存在：{file_id}")));
    }
    Ok((path, info))
}

/// 读取文本文件内容，返回 (文本, 元数据)。
pub fn read_text(
    store: &Store,
    file_id: &str,
    encoding: &str,
) -> AppResult<(String, TextFileInfo)> {
    let info = get_file(store, file_id)?;
    if !is_previewable(&info.extension) {
        return Err(AppError::invalid_parameter(format!(
            "{} 为二进制文档格式，不支持文本内容预览，请使用下载接口",
            info.extension
        )));
    }

    let path = store.upload_dir.join(&info.stored_name);
    if path.exists() {
        let bytes = fs::read(&path).map_err(|err| storage_io("读取文件失败", err))?;
        let text = decode_with(&bytes, encoding).ok_or_else(|| {
            AppError::invalid_parameter(format!("文件不是合法的 {encoding} 文本"))
        })?;
        return Ok((normalize_newlines(&text), info));
    }

    let chapter_rows: Vec<&Record> = store
        .contents
        .find_all()
        .iter()
        .filter(|row| str_field(row, "novel_id") == file_id)
        .collect();
    if chapter_rows.is_empty() {
        return Err(AppError::file_not_found(format!("文件不存在：{file_id}")));
    }

    let text = chapter_rows
        .iter()
        .map(|row| {
            if str_field(row, "chapter_title") == "全文" {
                str_field(row, "content").to_string()
            } else {
                format!(
                    "{}\n{}",
                    str_field(row, "chapter_title"),
                    str_field(row, "content")
                )
            }
        })
        .collect::<Vec<_>>()
        .join("\n\n");
    Ok((text, info))
}

fn decode_with(content: &[u8], encoding: &str) -> Option<String> {
    match encoding.to_lowercase().as_str() {
        "utf-8" | "utf8" => std::str::from_utf8(content)
            .ok()
            .map(|text| text.to_string()),
        "gb18030" => {
            let (text, _, _) = GB18030.decode(content);
            Some(text.into_owned())
        }
        "gbk" => {
            let (text, _, _) = GBK.decode(content);
            Some(text.into_owned())
        }
        _ => None,
    }
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

/// 按原始文件名或书名解析下载目标。
pub fn resolve_download(store: &Store, filename: &str) -> AppResult<(PathBuf, TextFileInfo)> {
    let safe_name = sanitize_filename(filename, 255)?;
    let title = Path::new(&safe_name)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("");
    for row in store.novels.find_all() {
        let row_title = str_field(row, "title");
        let format = str_field(row, "format");
        if row_title == title
            || format!("{row_title}.{format}") == safe_name
            || format!("{row_title}.txt") == safe_name
            || row_title == safe_name
        {
            let info = record_to_info(store, row);
            let path = store.upload_dir.join(&info.stored_name);
            if !path.exists() {
                return Err(AppError::file_not_found(format!("文件不存在：{filename}")));
            }
            return Ok((path, info));
        }
    }
    Err(AppError::file_not_found(format!("文件不存在：{filename}")))
}

pub fn get_novel_sentences(store: &Store, file_id: &str) -> Vec<Record> {
    store
        .sentences
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

/// 清理上传目录与 CSV 表（测试或运维使用）。
pub fn cleanup(store: &mut Store) -> AppResult<()> {
    if store.upload_dir.exists() {
        fs::remove_dir_all(&store.upload_dir).map_err(|err| storage_io("清理上传目录失败", err))?;
    }
    fs::create_dir_all(&store.upload_dir).map_err(|err| storage_io("创建上传目录失败", err))?;
    store.novels.clear();
    store.contents.clear();
    store.sentences.clear();
    store.polyphones.clear();
    store.save_all()
}

/// 兼容旧版索引接口。
pub fn load_index(store: &Store) -> Vec<Record> {
    store.novels.find_all().to_vec()
}

pub fn save_index(store: &mut Store, index: &[Record]) -> AppResult<()> {
    store.novels.clear();
    store.novels.insert_batch(index.to_vec());
    store.save_novels()
}

/// 兼容旧版布尔字段解析。
pub fn record_is_deleted(record: &Record) -> bool {
    bool_field(record, "is_deleted")
}
