pub mod chapters;
pub mod characters;
pub mod health;
pub mod polyphones;
pub mod projects;
pub mod recycle_bin;
pub mod schema;

use rmcp::model::Tool;

use super::config::McpSettings;

pub fn definitions(settings: &McpSettings) -> Vec<Tool> {
    let mut tools = vec![
        health::read_tools(),
        projects::read_tools(),
        chapters::read_tools(),
        polyphones::read_tools(),
        recycle_bin::read_tools(),
        characters::read_tools(),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();

    if settings.write_tools {
        tools.extend(projects::write_tools());
        tools.extend(chapters::write_tools());
        tools.extend(polyphones::write_tools());
        tools.extend(recycle_bin::write_tools());
        tools.extend(characters::write_tools());
    }
    if settings.destructive_tools {
        tools.extend(chapters::destructive_tools());
        tools.extend(polyphones::destructive_tools());
        tools.extend(recycle_bin::destructive_tools());
        tools.extend(characters::destructive_tools());
    }
    tools
}
