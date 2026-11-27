use std::collections::HashMap;

use chrono::Utc;

use crate::error::{AppError, AppResult};
use crate::storage::csv_table::{Record, str_field};
use crate::storage::store::Store;

use super::model::DialogueInfo;
use super::schema::{DialogueSpeakerBindRequest, DialogueSpeakerUnbindResponse};

pub struct DialogueListFilter<'a> {
    pub segment_type: Option<&'a str>,
    pub binding_status: Option<&'a str>,
    pub speaker_id: Option<&'a str>,
    pub keyword: Option<&'a str>,
}

pub fn list_dialogues(
    store: &Store,
    project_id: &str,
    chapter_id: &str,
    page: usize,
    page_size: usize,
    filter: &DialogueListFilter<'_>,
) -> AppResult<(Vec<DialogueInfo>, usize)> {
    let project = super::super::service::get_project(store, project_id, false)?;
    ensure_chapter(store, &project.file_id, chapter_id)?;
    validate_filter(filter)?;

    let speaker_names: HashMap<String, String> = store
        .characters
        .find_all()
        .iter()
        .filter(|record| str_field(record, "project_id") == project_id)
        .map(|record| {
            (
                str_field(record, "character_id").to_string(),
                str_field(record, "name").to_string(),
            )
        })
        .collect();
    let keyword = filter
        .keyword
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_lowercase);

    let mut items: Vec<DialogueInfo> = store
        .dialogues
        .find_all()
        .iter()
        .filter(|record| {
            str_field(record, "chapter_id") == chapter_id
                && str_field(record, "novel_id") == project.file_id
        })
        .filter(|record| {
            filter
                .segment_type
                .is_none_or(|value| str_field(record, "segment_type") == value)
        })
        .filter(|record| {
            let bound = !str_field(record, "speaker_id").trim().is_empty();
            match filter.binding_status {
                Some("bound") => bound,
                Some("unbound") => !bound,
                _ => true,
            }
        })
        .filter(|record| {
            filter
                .speaker_id
                .is_none_or(|value| str_field(record, "speaker_id") == value)
        })
        .filter(|record| {
            keyword.as_ref().is_none_or(|keyword| {
                str_field(record, "content")
                    .to_lowercase()
                    .contains(keyword)
            })
        })
        .map(|record| {
            let speaker_id = optional_field(record, "speaker_id");
            let speaker_name = speaker_id
                .as_ref()
                .and_then(|id| speaker_names.get(id))
                .cloned();
            record_to_info(record, project_id, speaker_name)
        })
        .collect();

    items.sort_by(|a, b| {
        a.sentence_index
            .cmp(&b.sentence_index)
            .then(a.segment_index.cmp(&b.segment_index))
    });
    let total = items.len();
    let start = page.saturating_sub(1).saturating_mul(page_size);
    let items = items.into_iter().skip(start).take(page_size).collect();
    Ok((items, total))
}

pub fn get_dialogue(
    store: &Store,
    project_id: &str,
    chapter_id: &str,
    dialogue_id: &str,
) -> AppResult<DialogueInfo> {
    let project = super::super::service::get_project(store, project_id, false)?;
    ensure_chapter(store, &project.file_id, chapter_id)?;
    let record = find_dialogue(store, &project.file_id, chapter_id, dialogue_id)?;
    let speaker_id = optional_field(record, "speaker_id");
    let speaker_name = speaker_id.as_ref().and_then(|id| {
        store
            .characters
            .find_all()
            .iter()
            .find(|character| {
                str_field(character, "project_id") == project_id
                    && str_field(character, "character_id") == id
            })
            .map(|character| str_field(character, "name").to_string())
    });
    Ok(record_to_info(record, project_id, speaker_name))
}

pub fn bind_speaker(
    store: &mut Store,
    project_id: &str,
    chapter_id: &str,
    dialogue_id: &str,
    request: &DialogueSpeakerBindRequest,
) -> AppResult<DialogueInfo> {
    let project = super::super::service::get_project(store, project_id, false)?;
    ensure_chapter(store, &project.file_id, chapter_id)?;
    find_dialogue(store, &project.file_id, chapter_id, dialogue_id)?;
    let speaker_id = request.speaker_id.trim();
    if speaker_id.is_empty() {
        return Err(AppError::invalid_parameter("说话人角色 ID 不能为空"));
    }
    super::super::characters::service::get_character(store, project_id, speaker_id)?;
    let source = request.binding_source.as_deref().unwrap_or("manual").trim();
    if !["manual", "auto", "imported"].contains(&source) {
        return Err(AppError::invalid_parameter(
            "绑定来源仅支持 manual / auto / imported",
        ));
    }
    if let Some(confidence) = request.binding_confidence
        && !(0.0..=1.0).contains(&confidence)
    {
        return Err(AppError::invalid_parameter(
            "匹配置信度必须在 0.0 到 1.0 之间",
        ));
    }

    store.dialogues.update_by(
        "dialogue_id",
        dialogue_id,
        &[
            ("speaker_id", speaker_id.to_string()),
            ("binding_source", source.to_string()),
            (
                "binding_confidence",
                request
                    .binding_confidence
                    .map(|value| value.to_string())
                    .unwrap_or_default(),
            ),
            ("speaker_updated_at", Utc::now().to_rfc3339()),
        ],
    );
    store.save_dialogues()?;
    get_dialogue(store, project_id, chapter_id, dialogue_id)
}

pub fn unbind_speaker(
    store: &mut Store,
    project_id: &str,
    chapter_id: &str,
    dialogue_id: &str,
) -> AppResult<DialogueSpeakerUnbindResponse> {
    let project = super::super::service::get_project(store, project_id, false)?;
    ensure_chapter(store, &project.file_id, chapter_id)?;
    find_dialogue(store, &project.file_id, chapter_id, dialogue_id)?;
    clear_binding(store, dialogue_id);
    store.save_dialogues()?;
    Ok(DialogueSpeakerUnbindResponse {
        dialogue_id: dialogue_id.to_string(),
        unbound: true,
    })
}

pub fn unbind_character(store: &mut Store, project_id: &str, character_id: &str) -> AppResult<()> {
    let dialogue_ids: Vec<String> = store
        .dialogues
        .find_all()
        .iter()
        .filter(|record| {
            str_field(record, "novel_id")
                == store
                    .projects
                    .find_by("project_id", project_id)
                    .map(|project| str_field(project, "file_id"))
                    .unwrap_or("")
                && str_field(record, "speaker_id") == character_id
        })
        .map(|record| str_field(record, "dialogue_id").to_string())
        .collect();
    if dialogue_ids.is_empty() {
        return Ok(());
    }
    let now = Utc::now().to_rfc3339();
    for dialogue_id in dialogue_ids {
        store.dialogues.update_by(
            "dialogue_id",
            &dialogue_id,
            &[
                ("speaker_id", String::new()),
                ("binding_source", "system".to_string()),
                ("binding_confidence", String::new()),
                ("speaker_updated_at", now.clone()),
            ],
        );
    }
    store.save_dialogues()
}

fn ensure_chapter(store: &Store, novel_id: &str, chapter_id: &str) -> AppResult<()> {
    store
        .contents
        .find_by("chapter_id", chapter_id)
        .filter(|record| str_field(record, "novel_id") == novel_id)
        .ok_or_else(|| AppError::file_not_found(format!("章节不存在：{chapter_id}")))?;
    Ok(())
}

fn find_dialogue<'a>(
    store: &'a Store,
    novel_id: &str,
    chapter_id: &str,
    dialogue_id: &str,
) -> AppResult<&'a Record> {
    store
        .dialogues
        .find_all()
        .iter()
        .find(|record| {
            str_field(record, "dialogue_id") == dialogue_id
                && str_field(record, "chapter_id") == chapter_id
                && str_field(record, "novel_id") == novel_id
        })
        .ok_or_else(|| AppError::file_not_found(format!("台词片段不存在：{dialogue_id}")))
}

fn clear_binding(store: &mut Store, dialogue_id: &str) {
    store.dialogues.update_by(
        "dialogue_id",
        dialogue_id,
        &[
            ("speaker_id", String::new()),
            ("binding_source", "system".to_string()),
            ("binding_confidence", String::new()),
            ("speaker_updated_at", Utc::now().to_rfc3339()),
        ],
    );
}

fn validate_filter(filter: &DialogueListFilter<'_>) -> AppResult<()> {
    if let Some(value) = filter.segment_type
        && !["narration", "dialogue"].contains(&value)
    {
        return Err(AppError::invalid_parameter(
            "片段类型仅支持 narration / dialogue",
        ));
    }
    if let Some(value) = filter.binding_status
        && !["all", "bound", "unbound"].contains(&value)
    {
        return Err(AppError::invalid_parameter(
            "绑定状态仅支持 all / bound / unbound",
        ));
    }
    Ok(())
}

fn record_to_info(record: &Record, project_id: &str, speaker_name: Option<String>) -> DialogueInfo {
    let speaker_id = optional_field(record, "speaker_id");
    let confidence_raw = str_field(record, "binding_confidence").trim();
    let updated_raw = str_field(record, "speaker_updated_at").trim();
    DialogueInfo {
        project_id: project_id.to_string(),
        chapter_id: str_field(record, "chapter_id").to_string(),
        dialogue_id: str_field(record, "dialogue_id").to_string(),
        parent_sentence_id: str_field(record, "parent_sentence_id").to_string(),
        sentence_index: str_field(record, "source_sentence_index")
            .parse()
            .unwrap_or(0),
        segment_index: str_field(record, "split_index").parse().unwrap_or(0),
        segment_type: str_field(record, "segment_type").to_string(),
        content: str_field(record, "content").to_string(),
        word_count: str_field(record, "word_count").parse().unwrap_or(0),
        parse_status: str_field(record, "parse_status").to_string(),
        speaker_id,
        speaker_name,
        binding_source: optional_field(record, "binding_source"),
        binding_confidence: (!confidence_raw.is_empty())
            .then(|| confidence_raw.parse::<f64>().ok())
            .flatten(),
        speaker_updated_at: (!updated_raw.is_empty())
            .then(|| super::super::storage_compat::parse_datetime(updated_raw)),
    }
}

fn optional_field(record: &Record, key: &str) -> Option<String> {
    let value = str_field(record, key).trim();
    (!value.is_empty()).then(|| value.to_string())
}
