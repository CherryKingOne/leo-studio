use leo_plugin_api::http::page_count;
use leo_plugin_api::modules::projects::characters::schema::{
    CharacterCreateRequest, CharacterListResponse, CharacterUpdateRequest,
};
use leo_plugin_api::modules::projects::characters::service::{
    self as character_service, CharacterListFilter,
};
use leo_plugin_api::storage::store::Store;

use super::CliResult;
use crate::cli::CharacterCommands;
use crate::output;

pub async fn run(store: &mut Store, command: CharacterCommands) -> CliResult<()> {
    match command {
        CharacterCommands::Create {
            project_id,
            name,
            aliases,
            gender,
            age,
            role_type,
            status,
            description,
            tags,
        } => {
            let request = CharacterCreateRequest {
                name,
                aliases,
                gender,
                age,
                role_type,
                status,
                description,
                tags,
            };
            let info = character_service::create_character(store, &project_id, &request)?;
            output::print_json(&info)
        }
        CharacterCommands::List {
            project_id,
            page,
            page_size,
            keyword,
            gender,
            role_type,
            status,
            tag,
            sort,
            order,
        } => {
            output::validate_pagination(page, page_size)?;
            output::validate_enum(
                gender.as_deref(),
                &["male", "female", "unknown", "other"],
                "性别",
            )?;
            output::validate_enum(
                role_type.as_deref(),
                &[
                    "protagonist",
                    "supporting",
                    "antagonist",
                    "narrator",
                    "other",
                ],
                "角色类型",
            )?;
            output::validate_enum(Some(&status), &["active", "archived", "all"], "角色状态")?;
            let status_filter = if status == "all" { None } else { Some(status) };
            let filter = CharacterListFilter {
                keyword: keyword.as_deref(),
                gender: gender.as_deref(),
                role_type: role_type.as_deref(),
                status: status_filter.as_deref(),
                tag: tag.as_deref(),
            };
            let (items, total) = character_service::list_characters(
                store,
                &project_id,
                page,
                page_size,
                &sort,
                &order,
                &filter,
            )?;
            output::print_json(&CharacterListResponse {
                project_id,
                total,
                page,
                page_size,
                pages: page_count(total, page_size),
                items,
            })
        }
        CharacterCommands::Get {
            project_id,
            character_id,
        } => {
            let info = character_service::get_character(store, &project_id, &character_id)?;
            output::print_json(&info)
        }
        CharacterCommands::Update {
            project_id,
            character_id,
            name,
            aliases,
            clear_aliases,
            gender,
            age,
            clear_age,
            role_type,
            status,
            description,
            clear_description,
            tags,
            clear_tags,
        } => {
            let request = CharacterUpdateRequest {
                name,
                aliases: if clear_aliases {
                    Some(Vec::new())
                } else if aliases.is_empty() {
                    None
                } else {
                    Some(aliases)
                },
                gender,
                age: if clear_age {
                    Some(serde_json::Value::Null)
                } else {
                    age.map(serde_json::Value::String)
                },
                role_type,
                status,
                description: if clear_description {
                    Some(String::new())
                } else {
                    description
                },
                tags: if clear_tags {
                    Some(Vec::new())
                } else if tags.is_empty() {
                    None
                } else {
                    Some(tags)
                },
            };
            let info =
                character_service::update_character(store, &project_id, &character_id, &request)?;
            output::print_json(&info)
        }
        CharacterCommands::Delete {
            project_id,
            character_id,
            confirm,
        } => {
            output::require_confirm(confirm, "删除角色")?;
            let info = character_service::delete_character(store, &project_id, &character_id)?;
            output::print_json(&info)
        }
    }
}
