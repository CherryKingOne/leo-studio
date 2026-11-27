use std::collections::HashSet;

use chrono::Utc;

use crate::error::{AppError, AppResult};
use crate::storage::csv_table::{Record, str_field};
use crate::storage::store::Store;

use super::model::CharacterInfo;
use super::schema::{CharacterCreateRequest, CharacterDeleteResponse, CharacterUpdateRequest};

const MAX_NAME_LEN: usize = 50;
const MAX_ALIAS_LEN: usize = 50;
const MAX_AGE_LEN: usize = 32;
const MAX_DESCRIPTION_LEN: usize = 2000;
const MAX_TAG_LEN: usize = 20;
const MAX_ARRAY_ITEMS: usize = 20;

pub struct CharacterListFilter<'a> {
    pub keyword: Option<&'a str>,
    pub gender: Option<&'a str>,
    pub role_type: Option<&'a str>,
    pub status: Option<&'a str>,
    pub tag: Option<&'a str>,
}

pub fn create_character(
    store: &mut Store,
    project_id: &str,
    request: &CharacterCreateRequest,
) -> AppResult<CharacterInfo> {
    let name = normalize_name(&request.name)?;
    let aliases = normalize_array(&request.aliases, MAX_ALIAS_LEN, "别名")?;
    ensure_distinct_names(&name, &aliases)?;
    let gender = normalize_enum(
        request.gender.as_deref(),
        "unknown",
        &["male", "female", "unknown", "other"],
        "性别",
    )?;
    let age = normalize_optional(request.age.as_deref(), MAX_AGE_LEN, "年龄")?;
    let role_type = normalize_enum(
        request.role_type.as_deref(),
        "supporting",
        &[
            "protagonist",
            "supporting",
            "antagonist",
            "narrator",
            "other",
        ],
        "角色类型",
    )?;
    let status = normalize_enum(
        request.status.as_deref(),
        "active",
        &["active", "archived"],
        "角色状态",
    )?;
    let description = normalize_description(request.description.as_deref())?;
    let tags = normalize_array(&request.tags, MAX_TAG_LEN, "标签")?;

    super::super::service::get_project(store, project_id, false)?;
    ensure_unique(store, project_id, &name, &aliases, None)?;

    let character_id = uuid::Uuid::new_v4().simple().to_string();
    let now = Utc::now();
    let now_iso = now.to_rfc3339();
    let mut record = Record::new();
    record.insert("character_id".into(), character_id);
    record.insert("project_id".into(), project_id.to_string());
    record.insert("name".into(), name);
    record.insert("aliases_json".into(), serialize_array(&aliases)?);
    record.insert("gender".into(), gender);
    record.insert("age".into(), age.unwrap_or_default());
    record.insert("role_type".into(), role_type);
    record.insert("status".into(), status);
    record.insert("description".into(), description);
    record.insert("tags_json".into(), serialize_array(&tags)?);
    record.insert("created_at".into(), now_iso.clone());
    record.insert("updated_at".into(), now_iso);
    store.characters.insert(record.clone());
    store.save_characters()?;
    Ok(record_to_info(&record))
}

pub fn list_characters(
    store: &Store,
    project_id: &str,
    page: usize,
    page_size: usize,
    sort: &str,
    order: &str,
    filter: &CharacterListFilter<'_>,
) -> AppResult<(Vec<CharacterInfo>, usize)> {
    super::super::service::get_project(store, project_id, false)?;
    let keyword = filter
        .keyword
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_lowercase);
    let tag = filter.tag.map(str::trim).filter(|value| !value.is_empty());

    let mut items: Vec<CharacterInfo> = store
        .characters
        .find_all()
        .iter()
        .filter(|record| str_field(record, "project_id") == project_id)
        .map(record_to_info)
        .filter(|item| filter.status.is_none_or(|status| item.status == status))
        .filter(|item| filter.gender.is_none_or(|gender| item.gender == gender))
        .filter(|item| {
            filter
                .role_type
                .is_none_or(|role_type| item.role_type == role_type)
        })
        .filter(|item| tag.is_none_or(|tag| item.tags.iter().any(|item_tag| item_tag == tag)))
        .filter(|item| {
            keyword.as_ref().is_none_or(|keyword| {
                item.name.to_lowercase().contains(keyword)
                    || item.description.to_lowercase().contains(keyword)
                    || item
                        .aliases
                        .iter()
                        .any(|alias| alias.to_lowercase().contains(keyword))
            })
        })
        .collect();

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
    Ok((items, total))
}

pub fn get_character(
    store: &Store,
    project_id: &str,
    character_id: &str,
) -> AppResult<CharacterInfo> {
    super::super::service::get_project(store, project_id, false)?;
    let record = store
        .characters
        .find_all()
        .iter()
        .find(|record| {
            str_field(record, "character_id") == character_id
                && str_field(record, "project_id") == project_id
        })
        .ok_or_else(|| AppError::file_not_found(format!("角色不存在：{character_id}")))?;
    Ok(record_to_info(record))
}

pub fn update_character(
    store: &mut Store,
    project_id: &str,
    character_id: &str,
    request: &CharacterUpdateRequest,
) -> AppResult<CharacterInfo> {
    if !request.has_updates() {
        return Err(AppError::invalid_parameter("请提供要更新的字段"));
    }
    super::super::service::get_project(store, project_id, false)?;
    let current = get_character(store, project_id, character_id)?;

    let name = match request.name.as_deref() {
        Some(value) => normalize_name(value)?,
        None => current.name.clone(),
    };
    let aliases = match request.aliases.as_ref() {
        Some(value) => normalize_array(value, MAX_ALIAS_LEN, "别名")?,
        None => current.aliases.clone(),
    };
    ensure_distinct_names(&name, &aliases)?;
    let gender = match request.gender.as_deref() {
        Some(value) => normalize_enum(
            Some(value),
            "unknown",
            &["male", "female", "unknown", "other"],
            "性别",
        )?,
        None => current.gender.clone(),
    };
    let age = match request.age.as_ref() {
        Some(serde_json::Value::Null) => None,
        Some(serde_json::Value::String(value)) => {
            normalize_optional(Some(value), MAX_AGE_LEN, "年龄")?
        }
        Some(_) => {
            return Err(AppError::invalid_parameter("年龄必须为字符串或 null"));
        }
        None => current.age.clone(),
    };
    let role_type = match request.role_type.as_deref() {
        Some(value) => normalize_enum(
            Some(value),
            "supporting",
            &[
                "protagonist",
                "supporting",
                "antagonist",
                "narrator",
                "other",
            ],
            "角色类型",
        )?,
        None => current.role_type.clone(),
    };
    let status = match request.status.as_deref() {
        Some(value) => normalize_enum(Some(value), "active", &["active", "archived"], "角色状态")?,
        None => current.status.clone(),
    };
    let description = match request.description.as_deref() {
        Some(value) => normalize_description(Some(value))?,
        None => current.description.clone(),
    };
    let tags = match request.tags.as_ref() {
        Some(value) => normalize_array(value, MAX_TAG_LEN, "标签")?,
        None => current.tags.clone(),
    };

    ensure_unique(store, project_id, &name, &aliases, Some(character_id))?;
    let now = Utc::now().to_rfc3339();
    store.characters.update_by(
        "character_id",
        character_id,
        &[
            ("name", name),
            ("aliases_json", serialize_array(&aliases)?),
            ("gender", gender),
            ("age", age.unwrap_or_default()),
            ("role_type", role_type),
            ("status", status),
            ("description", description),
            ("tags_json", serialize_array(&tags)?),
            ("updated_at", now),
        ],
    );
    store.save_characters()?;
    get_character(store, project_id, character_id)
}

pub fn delete_character(
    store: &mut Store,
    project_id: &str,
    character_id: &str,
) -> AppResult<CharacterDeleteResponse> {
    let character = get_character(store, project_id, character_id)?;
    store.characters.delete_by("character_id", character_id);
    store.save_characters()?;
    Ok(CharacterDeleteResponse {
        project_id: project_id.to_string(),
        character_id: character.character_id,
        name: character.name,
        deleted: true,
    })
}

pub fn record_to_info(record: &Record) -> CharacterInfo {
    let created_at = super::super::storage_compat::parse_datetime(str_field(record, "created_at"));
    let updated_at_raw = str_field(record, "updated_at");
    let updated_at = if updated_at_raw.is_empty() {
        created_at
    } else {
        super::super::storage_compat::parse_datetime(updated_at_raw)
    };
    let age_raw = str_field(record, "age").trim();
    CharacterInfo {
        character_id: str_field(record, "character_id").to_string(),
        project_id: str_field(record, "project_id").to_string(),
        name: str_field(record, "name").to_string(),
        aliases: parse_array(str_field(record, "aliases_json")),
        gender: str_field(record, "gender").to_string(),
        age: (!age_raw.is_empty()).then(|| age_raw.to_string()),
        role_type: str_field(record, "role_type").to_string(),
        status: str_field(record, "status").to_string(),
        description: str_field(record, "description").to_string(),
        tags: parse_array(str_field(record, "tags_json")),
        created_at,
        updated_at,
    }
}

fn normalize_name(value: &str) -> AppResult<String> {
    normalize_required(value, MAX_NAME_LEN, "角色名称")
}

fn normalize_required(value: &str, max_len: usize, field: &str) -> AppResult<String> {
    let value = value.trim();
    if value.is_empty() {
        return Err(AppError::invalid_parameter(format!("{field}不能为空")));
    }
    if value.chars().count() > max_len {
        return Err(AppError::invalid_parameter(format!(
            "{field}长度不能超过 {max_len} 个字符"
        )));
    }
    Ok(value.to_string())
}

fn normalize_optional(
    value: Option<&str>,
    max_len: usize,
    field: &str,
) -> AppResult<Option<String>> {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    if value.chars().count() > max_len {
        return Err(AppError::invalid_parameter(format!(
            "{field}长度不能超过 {max_len} 个字符"
        )));
    }
    Ok(Some(value.to_string()))
}

fn normalize_description(value: Option<&str>) -> AppResult<String> {
    let value = value.unwrap_or("").trim();
    if value.chars().count() > MAX_DESCRIPTION_LEN {
        return Err(AppError::invalid_parameter(format!(
            "角色简介长度不能超过 {MAX_DESCRIPTION_LEN} 个字符"
        )));
    }
    Ok(value.to_string())
}

fn normalize_array(values: &[String], max_len: usize, field: &str) -> AppResult<Vec<String>> {
    if values.len() > MAX_ARRAY_ITEMS {
        return Err(AppError::invalid_parameter(format!(
            "{field}数量不能超过 {MAX_ARRAY_ITEMS} 个"
        )));
    }
    let mut seen = HashSet::new();
    let mut result = Vec::new();
    for value in values {
        let value = normalize_required(value, max_len, field)?;
        if seen.insert(value.to_lowercase()) {
            result.push(value);
        }
    }
    Ok(result)
}

fn normalize_enum(
    value: Option<&str>,
    default: &str,
    allowed: &[&str],
    field: &str,
) -> AppResult<String> {
    let value = value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(default);
    if !allowed.contains(&value) {
        return Err(AppError::invalid_parameter(format!(
            "{field}不支持 {value}，可选值：{}",
            allowed.join(", ")
        )));
    }
    Ok(value.to_string())
}

fn ensure_distinct_names(name: &str, aliases: &[String]) -> AppResult<()> {
    let name_key = name.to_lowercase();
    if aliases.iter().any(|alias| alias.to_lowercase() == name_key) {
        return Err(AppError::file_conflict("角色名称不能与自己的别名冲突"));
    }
    Ok(())
}

fn ensure_unique(
    store: &Store,
    project_id: &str,
    name: &str,
    aliases: &[String],
    exclude_id: Option<&str>,
) -> AppResult<()> {
    let mut existing = HashSet::new();
    for record in store
        .characters
        .find_all()
        .iter()
        .filter(|record| str_field(record, "project_id") == project_id)
        .filter(|record| exclude_id.is_none_or(|id| str_field(record, "character_id") != id))
    {
        let existing_name = str_field(record, "name").trim();
        if !existing_name.is_empty() {
            existing.insert(existing_name.to_lowercase());
        }
        for alias in parse_array(str_field(record, "aliases_json")) {
            existing.insert(alias.to_lowercase());
        }
    }

    let mut candidate = vec![name.to_lowercase()];
    candidate.extend(aliases.iter().map(|alias| alias.to_lowercase()));
    if candidate.iter().any(|value| existing.contains(value)) {
        return Err(AppError::file_conflict("项目内已存在同名角色或别名"));
    }
    Ok(())
}

fn serialize_array(values: &[String]) -> AppResult<String> {
    serde_json::to_string(values)
        .map_err(|err| AppError::internal(format!("序列化角色数组失败：{err}")))
}

fn parse_array(value: &str) -> Vec<String> {
    if value.trim().is_empty() {
        return Vec::new();
    }
    serde_json::from_str(value).unwrap_or_default()
}
