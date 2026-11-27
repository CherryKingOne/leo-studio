use std::fs;

use leo_plugin_api::modules::projects::schema::{
    PolyphoneAnnotation, SentencePolyphoneListResponse,
};
use leo_plugin_api::modules::projects::service as project_service;
use leo_plugin_api::storage::store::Store;

use super::CliResult;
use crate::cli::PolyphoneCommands;
use crate::output;

pub async fn run(store: &mut Store, command: PolyphoneCommands) -> CliResult<()> {
    match command {
        PolyphoneCommands::ChapterList {
            project_id,
            chapter_id,
        } => {
            let info = project_service::get_chapter_polyphones(store, &project_id, &chapter_id)?;
            output::print_json(&info)
        }
        PolyphoneCommands::SentenceList {
            project_id,
            chapter_id,
            sentence_id,
        } => {
            let items = project_service::get_chapter_sentence_polyphones(
                store,
                &project_id,
                &chapter_id,
                &sentence_id,
            )?;
            output::print_json(&SentencePolyphoneListResponse {
                sentence_id,
                total: items.len(),
                items,
            })
        }
        PolyphoneCommands::Save {
            project_id,
            chapter_id,
            sentence_id,
            annotations_file,
        } => {
            let content = fs::read_to_string(&annotations_file)?;
            let annotations: Vec<PolyphoneAnnotation> = serde_json::from_str(&content)?;
            let items = project_service::save_chapter_sentence_polyphones(
                store,
                &project_id,
                &chapter_id,
                &sentence_id,
                &annotations,
            )?;
            output::print_json(&SentencePolyphoneListResponse {
                sentence_id,
                total: items.len(),
                items,
            })
        }
        PolyphoneCommands::Clear {
            project_id,
            chapter_id,
            sentence_id,
            confirm,
        } => {
            output::require_confirm(confirm, "清空多音字标注")?;
            let deleted_count = project_service::delete_chapter_sentence_polyphones(
                store,
                &project_id,
                &chapter_id,
                &sentence_id,
            )?;
            output::print_json(&serde_json::json!({
                "sentence_id": sentence_id,
                "deleted_count": deleted_count,
                "deleted": true
            }))
        }
        PolyphoneCommands::DeleteItem {
            project_id,
            chapter_id,
            sentence_id,
            char_index,
            confirm,
        } => {
            output::require_confirm(confirm, "删除单条多音字标注")?;
            let info = project_service::delete_chapter_sentence_polyphone(
                store,
                &project_id,
                &chapter_id,
                &sentence_id,
                char_index,
            )?;
            output::print_json(&info)
        }
    }
}
