use std::fs;

use leo_plugin_cli::cli::{CharacterCommands, DialogueCommands, ProjectCommands};
use leo_plugin_cli::commands::{characters, dialogues, projects};
use leo_plugin_cli::runtime::build_store;

#[tokio::test]
async fn project_and_character_commands_use_v3_store() {
    let dir = tempfile::tempdir().unwrap();
    let novel = dir.path().join("novel.txt");
    fs::write(&novel, "第1章 开端\n林冲说：“武松。”").unwrap();
    let (settings, mut store) = build_store(dir.path(), None).unwrap();

    projects::run(
        &settings,
        &mut store,
        ProjectCommands::Create {
            name: "CLI 项目".into(),
            file: novel,
        },
    )
    .await
    .unwrap();
    let project_id = store.projects.find_all()[0]
        .get("project_id")
        .unwrap()
        .clone();

    characters::run(
        &mut store,
        CharacterCommands::Create {
            project_id: project_id.clone(),
            name: "林冲".into(),
            aliases: vec!["豹子头".into()],
            gender: Some("male".into()),
            age: Some("三十出头".into()),
            role_type: Some("protagonist".into()),
            status: None,
            description: None,
            tags: vec!["武将".into()],
        },
    )
    .await
    .unwrap();
    assert_eq!(store.characters.find_all().len(), 1);
    let character_id = store.characters.find_all()[0]
        .get("character_id")
        .unwrap()
        .clone();
    let chapter_id = store.contents.find_all()[0]
        .get("chapter_id")
        .unwrap()
        .clone();
    let dialogue_id = store.dialogues.find_all()[0]
        .get("dialogue_id")
        .unwrap()
        .clone();
    dialogues::run(
        &mut store,
        DialogueCommands::Bind {
            project_id: project_id.clone(),
            chapter_id: chapter_id.clone(),
            dialogue_id: dialogue_id.clone(),
            speaker_id: character_id.clone(),
            binding_source: "manual".into(),
            binding_confidence: None,
        },
    )
    .await
    .unwrap();
    assert_eq!(
        store.dialogues.find_all()[0].get("speaker_id").unwrap(),
        &character_id
    );
    dialogues::run(
        &mut store,
        DialogueCommands::Unbind {
            project_id: project_id.clone(),
            chapter_id,
            dialogue_id,
        },
    )
    .await
    .unwrap();
    assert!(
        store.dialogues.find_all()[0]
            .get("speaker_id")
            .unwrap()
            .is_empty()
    );

    characters::run(
        &mut store,
        CharacterCommands::Update {
            project_id,
            character_id: character_id.clone(),
            name: None,
            aliases: vec![],
            clear_aliases: false,
            gender: None,
            age: None,
            clear_age: true,
            role_type: None,
            status: Some("archived".into()),
            description: None,
            clear_description: false,
            tags: vec![],
            clear_tags: true,
        },
    )
    .await
    .unwrap();
    let record = &store.characters.find_all()[0];
    assert_eq!(record.get("status").unwrap(), "archived");
    assert_eq!(record.get("age").unwrap(), "");
    let project_id = record.get("project_id").unwrap().clone();

    let error = characters::run(
        &mut store,
        CharacterCommands::Delete {
            project_id,
            character_id,
            confirm: false,
        },
    )
    .await
    .unwrap_err();
    assert!(error.to_string().contains("--confirm"));
}
