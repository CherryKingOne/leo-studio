use leo_plugin_api::http::page_count;
use leo_plugin_api::modules::projects::dialogues::schema::{
    DialogueListResponse, DialogueSpeakerBindRequest,
};
use leo_plugin_api::modules::projects::dialogues::service::{
    self as dialogue_service, DialogueListFilter,
};
use leo_plugin_api::storage::store::Store;

use super::CliResult;
use crate::cli::DialogueCommands;
use crate::output;

pub async fn run(store: &mut Store, command: DialogueCommands) -> CliResult<()> {
    match command {
        DialogueCommands::List {
            project_id,
            chapter_id,
            page,
            page_size,
            segment_type,
            binding_status,
            speaker_id,
            keyword,
        } => {
            output::validate_pagination(page, page_size)?;
            output::validate_enum(
                segment_type.as_deref(),
                &["narration", "dialogue"],
                "片段类型",
            )?;
            output::validate_enum(
                Some(&binding_status),
                &["all", "bound", "unbound"],
                "绑定状态",
            )?;
            let filter = DialogueListFilter {
                segment_type: segment_type.as_deref(),
                binding_status: (binding_status != "all").then_some(binding_status.as_str()),
                speaker_id: speaker_id.as_deref(),
                keyword: keyword.as_deref(),
            };
            let (items, total) = dialogue_service::list_dialogues(
                store,
                &project_id,
                &chapter_id,
                page,
                page_size,
                &filter,
            )?;
            output::print_json(&DialogueListResponse {
                project_id,
                chapter_id,
                total,
                page,
                page_size,
                pages: page_count(total, page_size),
                items,
            })
        }
        DialogueCommands::Get {
            project_id,
            chapter_id,
            dialogue_id,
        } => {
            let dialogue =
                dialogue_service::get_dialogue(store, &project_id, &chapter_id, &dialogue_id)?;
            output::print_json(&dialogue)
        }
        DialogueCommands::Bind {
            project_id,
            chapter_id,
            dialogue_id,
            speaker_id,
            binding_source,
            binding_confidence,
        } => {
            output::validate_enum(
                Some(&binding_source),
                &["manual", "auto", "imported"],
                "绑定来源",
            )?;
            let request = DialogueSpeakerBindRequest {
                speaker_id,
                binding_source: Some(binding_source),
                binding_confidence,
            };
            let dialogue = dialogue_service::bind_speaker(
                store,
                &project_id,
                &chapter_id,
                &dialogue_id,
                &request,
            )?;
            output::print_json(&dialogue)
        }
        DialogueCommands::Unbind {
            project_id,
            chapter_id,
            dialogue_id,
        } => {
            let result =
                dialogue_service::unbind_speaker(store, &project_id, &chapter_id, &dialogue_id)?;
            output::print_json(&result)
        }
    }
}
