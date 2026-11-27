use leo_plugin_api::config::Settings;
use leo_plugin_api::http::page_count;
use leo_plugin_api::modules::projects::schema::ProjectListResponse;
use leo_plugin_api::modules::projects::service as project_service;
use leo_plugin_api::storage::store::Store;

use super::CliResult;
use crate::cli::RecycleBinCommands;
use crate::output;

pub async fn run(
    _settings: &Settings,
    store: &mut Store,
    command: RecycleBinCommands,
) -> CliResult<()> {
    match command {
        RecycleBinCommands::List { page, page_size } => {
            output::validate_pagination(page, page_size)?;
            let (items, total) = project_service::list_projects(store, page, page_size, true);
            output::print_json(&ProjectListResponse {
                total,
                page,
                page_size,
                pages: page_count(total, page_size),
                items,
            })
        }
        RecycleBinCommands::Clear { confirm } => {
            output::require_confirm(confirm, "清空回收站")?;
            let cleared_count = project_service::clear_recycle_bin(store)?;
            output::print_json(&serde_json::json!({
                "cleared_count": cleared_count,
                "cleared": true
            }))
        }
        RecycleBinCommands::Restore { project_id } => {
            let info = project_service::restore_project(store, &project_id)?;
            output::print_json(&info)
        }
        RecycleBinCommands::Purge {
            project_id,
            confirm,
        } => {
            output::require_confirm(confirm, "彻底删除项目")?;
            let info = project_service::hard_delete_project(store, &project_id)?;
            output::print_json(&info)
        }
    }
}
