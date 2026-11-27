use std::fs;

use leo_plugin_api::config::Settings;
use leo_plugin_api::modules::projects::schema::ChapterOutlineListResponse;
use leo_plugin_api::modules::projects::service as project_service;
use leo_plugin_api::storage::store::Store;

use super::CliResult;
use crate::cli::ChapterCommands;
use crate::output;

pub async fn run(
    settings: &Settings,
    store: &mut Store,
    command: ChapterCommands,
) -> CliResult<()> {
    match command {
        ChapterCommands::List {
            project_id,
            keyword,
        } => {
            let mut items = project_service::get_project_chapters(store, &project_id)?;
            if let Some(keyword) = keyword
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                items.retain(|item| item.chapter_title.contains(keyword));
            }
            let total = items.len();
            output::print_json(&ChapterOutlineListResponse {
                project_id,
                total,
                items,
            })
        }
        ChapterCommands::Append {
            project_id,
            file,
            overwrite_existing,
            dry_run,
            confirm,
        } => {
            if !dry_run {
                output::require_confirm(confirm, "追更写入")?;
            }
            let content = fs::read(&file)?;
            let filename = file
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("chapters.txt");
            let info = project_service::append_chapters(
                store,
                settings,
                &project_id,
                filename,
                &content,
                None,
                overwrite_existing,
                dry_run,
            )?;
            output::print_json(&info)
        }
        ChapterCommands::Get {
            project_id,
            chapter_id,
        } => {
            let info =
                project_service::get_project_chapter_content(store, &project_id, &chapter_id)?;
            output::print_json(&info)
        }
        ChapterCommands::Delete {
            project_id,
            chapter_id,
            confirm,
        } => {
            output::require_confirm(confirm, "删除章节")?;
            let info = project_service::delete_chapter(store, &project_id, &chapter_id)?;
            output::print_json(&info)
        }
    }
}
