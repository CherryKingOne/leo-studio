use std::path::PathBuf;

use clap::Parser;
use leo_core::plugin::{Plugin, PluginCategory};
use leo_plugin_cli::{
    CharacterCommands, Cli, CliPlugin, Commands, ProjectCommands, RecycleBinCommands,
    setup_cli_runtime,
};

#[test]
fn test_cli_plugin_manifest() {
    let plugin = CliPlugin::new();
    let manifest = plugin.manifest();
    assert_eq!(manifest.id, "leo-plugin-cli");
    assert_eq!(manifest.category, PluginCategory::Cli);
    assert!(manifest.capabilities.contains(&"cli:projects".to_string()));
    assert!(
        manifest
            .capabilities
            .contains(&"cli:characters".to_string())
    );
    assert!(manifest.capabilities.contains(&"cli:import".to_string()));
    assert!(
        manifest
            .capabilities
            .contains(&"cli:synthesize".to_string())
    );
}

#[tokio::test]
async fn test_cli_runtime_setup() {
    let temp_dir = tempfile::tempdir().unwrap();
    let data_dir = temp_dir.path().to_str().unwrap();

    let (_ctx, manager) = setup_cli_runtime(data_dir)
        .await
        .expect("装配 CLI 环境失败");
    let manifests = manager.list_manifests().await;
    assert!(manifests.len() >= 8);
    assert!(manifests.iter().any(|m| m.id == "leo-plugin-cli"));
    assert!(manifests.iter().any(|m| m.id == "leo-plugin-tts-edge"));
    assert!(manifests.iter().any(|m| m.id == "leo-plugin-importer-txt"));
}

#[test]
fn test_cli_command_parsing() {
    let cli = Cli::try_parse_from([
        "leo-cli",
        "import",
        "sample.txt",
        "--title",
        "自定义仙侠小说",
        "--data-dir",
        "./data",
    ])
    .unwrap();
    assert_eq!(cli.data_dir, PathBuf::from("./data"));
    match cli.command {
        Commands::Import { file, title } => {
            assert_eq!(file, PathBuf::from("sample.txt"));
            assert_eq!(title, Some("自定义仙侠小说".to_string()));
        }
        _ => panic!("解析命令类型错误"),
    }

    let cli = Cli::try_parse_from([
        "leo-cli", "export", "proj_123", "--format", "json", "-o", "out.json",
    ])
    .unwrap();
    match cli.command {
        Commands::Export {
            project_id,
            format,
            output,
        } => {
            assert_eq!(project_id, "proj_123");
            assert_eq!(format, "json");
            assert_eq!(output, Some(PathBuf::from("out.json")));
        }
        _ => panic!("解析命令类型错误"),
    }

    let cli = Cli::try_parse_from([
        "leo-cli",
        "synthesize",
        "proj_123",
        "--chapter",
        "1",
        "--voice",
        "zh-CN-XiaoxiaoNeural",
        "--speed",
        "1.2",
    ])
    .unwrap();
    match cli.command {
        Commands::Synthesize {
            project_id,
            chapter,
            voice,
            speed,
            ..
        } => {
            assert_eq!(project_id, "proj_123");
            assert_eq!(chapter, 1);
            assert_eq!(voice, "zh-CN-XiaoxiaoNeural");
            assert_eq!(speed, 1.2);
        }
        _ => panic!("解析命令类型错误"),
    }
}

#[test]
fn test_v3_project_and_character_command_parsing() {
    let cli = Cli::try_parse_from([
        "leo-cli",
        "project",
        "characters",
        "create",
        "proj_123",
        "--name",
        "林冲",
        "--alias",
        "豹子头",
        "--role-type",
        "protagonist",
        "--tag",
        "武将",
    ])
    .unwrap();
    match cli.command {
        Commands::Project {
            command:
                ProjectCommands::Characters {
                    command:
                        CharacterCommands::Create {
                            project_id,
                            name,
                            aliases,
                            role_type,
                            tags,
                            ..
                        },
                },
        } => {
            assert_eq!(project_id, "proj_123");
            assert_eq!(name, "林冲");
            assert_eq!(aliases, vec!["豹子头"]);
            assert_eq!(role_type.as_deref(), Some("protagonist"));
            assert_eq!(tags, vec!["武将"]);
        }
        _ => panic!("解析角色命令类型错误"),
    }

    let cli = Cli::try_parse_from([
        "leo-cli",
        "project",
        "characters",
        "update",
        "proj_123",
        "char_123",
        "--clear-age",
        "--clear-tags",
    ])
    .unwrap();
    match cli.command {
        Commands::Project {
            command:
                ProjectCommands::Characters {
                    command:
                        CharacterCommands::Update {
                            clear_age,
                            clear_tags,
                            ..
                        },
                },
        } => {
            assert!(clear_age);
            assert!(clear_tags);
        }
        _ => panic!("解析角色更新命令类型错误"),
    }
}

#[test]
fn test_recycle_bin_command_parsing() {
    let cli =
        Cli::try_parse_from(["leo-cli", "recycle-bin", "purge", "proj_123", "--confirm"]).unwrap();
    match cli.command {
        Commands::RecycleBin {
            command:
                RecycleBinCommands::Purge {
                    project_id,
                    confirm,
                },
        } => {
            assert_eq!(project_id, "proj_123");
            assert!(confirm);
        }
        _ => panic!("解析回收站命令类型错误"),
    }
}
