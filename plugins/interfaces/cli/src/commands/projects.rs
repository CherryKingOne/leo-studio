use std::fs;

use leo_plugin_api::config::Settings;
use leo_plugin_api::http::page_count;
use leo_plugin_api::modules::projects::schema::ProjectListResponse;
use leo_plugin_api::modules::projects::service as project_service;
use leo_plugin_api::storage::store::Store;

use super::CliResult;
use crate::cli::ProjectCommands;
use crate::output;

pub async fn run(
    settings: &Settings,
    store: &mut Store,
    command: ProjectCommands,
) -> CliResult<()> {
    match command {
        ProjectCommands::Create { name, file } => {
            let content = fs::read(&file)?;
            let filename = file
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("novel.txt");
            let info =
                project_service::create_project(store, settings, &name, filename, &content, None)?;
            output::print_json(&info)
        }
        ProjectCommands::List {
            page,
            page_size,
            status,
            keyword,
            sort,
            order,
        } => {
            output::validate_pagination(page, page_size)?;
            let (items, total) = project_service::query_projects(
                store,
                page,
                page_size,
                false,
                status.as_deref(),
                keyword.as_deref(),
                &sort,
                &order,
            );
            output::print_json(&ProjectListResponse {
                total,
                page,
                page_size,
                pages: page_count(total, page_size),
                items,
            })
        }
        ProjectCommands::Get { project_id } => {
            let info = project_service::get_project(store, &project_id, false)?;
            output::print_json(&info)
        }
        ProjectCommands::Update {
            project_id,
            name,
            status,
        } => {
            let info = project_service::update_project_fields(
                store,
                &project_id,
                name.as_deref(),
                status.as_deref(),
            )?;
            output::print_json(&info)
        }
        ProjectCommands::Delete {
            project_id,
            confirm,
        } => {
            output::require_confirm(confirm, "移入回收站")?;
            let info = project_service::delete_project(store, &project_id)?;
            output::print_json(&info)
        }
        ProjectCommands::Chapters { command } => {
            super::chapters::run(settings, store, command).await
        }
        ProjectCommands::Dialogues { command } => super::dialogues::run(store, command).await,
        ProjectCommands::Characters { command } => super::characters::run(store, command).await,
        ProjectCommands::Polyphones { command } => super::polyphones::run(store, command).await,
    }
}
