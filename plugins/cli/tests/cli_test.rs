use clap::Parser;
use leo_core::plugin::{Plugin, PluginCategory};
use leo_plugin_cli::{Cli, CliPlugin, Commands, setup_cli_runtime};

#[test]
fn test_cli_plugin_manifest() {
    let plugin = CliPlugin::new();
    let manifest = plugin.manifest();
    assert_eq!(manifest.id, "leo-plugin-cli");
    assert_eq!(manifest.category, PluginCategory::Cli);
    assert!(manifest.capabilities.contains(&"cli:import".to_string()));
    assert!(manifest.capabilities.contains(&"cli:synthesize".to_string()));
}

#[tokio::test]
async fn test_cli_runtime_setup() {
    let temp_dir = tempfile::tempdir().unwrap();
    let data_dir = temp_dir.path().to_str().unwrap();

    let (_ctx, manager) = setup_cli_runtime(data_dir).await.expect("装配 CLI 环境失败");
    let manifests = manager.list_manifests().await;
    assert!(manifests.len() >= 8);
    assert!(manifests.iter().any(|m| m.id == "leo-plugin-cli"));
    assert!(manifests.iter().any(|m| m.id == "leo-plugin-tts-edge"));
    assert!(manifests.iter().any(|m| m.id == "leo-plugin-importer-txt"));
}

#[test]
fn test_cli_command_parsing() {
    // 1. import
    let cli = Cli::try_parse_from(["leo-cli", "import", "sample.txt", "--title", "自定义仙侠小说"]).unwrap();
    match cli.command {
        Commands::Import { file, title } => {
            assert_eq!(file, "sample.txt");
            assert_eq!(title, Some("自定义仙侠小说".to_string()));
        }
        _ => panic!("解析命令类型错误"),
    }

    // 2. export
    let cli = Cli::try_parse_from(["leo-cli", "export", "proj_123", "--format", "json", "-o", "out.json"]).unwrap();
    match cli.command {
        Commands::Export { project_id, format, output } => {
            assert_eq!(project_id, "proj_123");
            assert_eq!(format, "json");
            assert_eq!(output, Some("out.json".to_string()));
        }
        _ => panic!("解析命令类型错误"),
    }

    // 3. synthesize
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
        Commands::Synthesize { project_id, chapter, voice, speed, .. } => {
            assert_eq!(project_id, "proj_123");
            assert_eq!(chapter, 1);
            assert_eq!(voice, "zh-CN-XiaoxiaoNeural");
            assert_eq!(speed, 1.2);
        }
        _ => panic!("解析命令类型错误"),
    }
}

#[tokio::test]
async fn test_cli_runtime_keeps_both_exporters() {
    use std::sync::Arc;

    use leo_core::plugin::Exporter;

    let temp_dir = tempfile::tempdir().unwrap();
    let (ctx, _manager) = setup_cli_runtime(temp_dir.path().to_str().unwrap())
        .await
        .expect("装配 CLI 环境失败");

    let names: Vec<String> = ctx
        .inject_all::<Arc<dyn Exporter>>()
        .await
        .into_iter()
        .map(|(name, _)| name)
        .collect();
    assert_eq!(names, vec!["csv", "json"]);
    for format in ["csv", "json"] {
        let exporter = ctx.inject_named::<Arc<dyn Exporter>>(format).await.unwrap();
        assert_eq!(exporter.format_name(), format);
    }
}

#[tokio::test]
async fn test_cli_export_routes_through_exporter_plugins() {
    use std::sync::Arc;

    use leo_core::plugin::{Importer, StorageProvider};
    use leo_plugin_cli::execute_cli;

    let temp_dir = tempfile::tempdir().unwrap();
    let data_dir = temp_dir.path().join("data").to_str().unwrap().to_string();

    let doc = {
        let (ctx, _manager) = setup_cli_runtime(&data_dir).await.expect("装配 CLI 环境失败");
        let importer = ctx.inject::<Arc<dyn Importer>>().await.unwrap();
        let doc = importer
            .import("t.txt", "第一章 开端\n他说：“你好。”".as_bytes())
            .await
            .unwrap();
        let storage = ctx.inject::<Arc<dyn StorageProvider>>().await.unwrap();
        storage.save_script(&doc).await.unwrap();
        doc
    };

    for (format, needle) in [("csv", "章节序号"), ("JSON", "\"chapters\"")] {
        let output = temp_dir.path().join(format!("out.{format}"));
        let cli = Cli {
            command: Commands::Export {
                project_id: doc.id.clone(),
                format: format.to_string(),
                output: Some(output.to_str().unwrap().to_string()),
            },
            data_dir: data_dir.clone(),
        };
        execute_cli(cli).await.expect("导出失败");
        let text = std::fs::read_to_string(&output).unwrap();
        assert!(text.contains(needle), "{format}: {text}");
    }

    let cli = Cli {
        command: Commands::Export {
            project_id: doc.id.clone(),
            format: "xml".to_string(),
            output: Some(temp_dir.path().join("out.xml").to_str().unwrap().to_string()),
        },
        data_dir,
    };
    let err = execute_cli(cli).await.expect_err("未注册的导出格式应报错");
    assert!(err.to_string().contains("csv, json"), "{err}");
}

mod fake_tts {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use leo_core::ast::AudioFormat;
    use leo_core::plugin::{SynthesisOutput, SynthesisPayload, TTSProvider, VoiceInfo};

    /// 离线假 TTS：输出 10 帧合法 MP3（MPEG-2 Layer III 24kHz，每帧 24ms，共 240ms）
    pub struct FakeTts {
        pub calls: AtomicUsize,
    }

    #[async_trait::async_trait]
    impl TTSProvider for FakeTts {
        fn provider_id(&self) -> &str {
            "fake-tts"
        }

        async fn list_voices(&self) -> leo_core::CoreResult<Vec<VoiceInfo>> {
            Ok(Vec::new())
        }

        async fn synthesize(&self, _payload: &SynthesisPayload) -> leo_core::CoreResult<SynthesisOutput> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            let mut frame = vec![0u8; 144];
            frame[..4].copy_from_slice(&[0xFF, 0xF3, 0x64, 0xC4]);
            Ok(SynthesisOutput {
                audio_bytes: frame.repeat(10),
                format: AudioFormat::Mp3,
                sample_rate: 24_000,
                duration_ms: 1,
            })
        }
    }
}

async fn runtime_with_fake_tts(
    data_dir: &str,
    fake: std::sync::Arc<fake_tts::FakeTts>,
) -> (leo_core::Context, std::sync::Arc<leo_core::plugin::PluginManager>) {
    use std::sync::Arc;

    use leo_core::plugin::TTSProvider;

    let (ctx, manager) = setup_cli_runtime(data_dir).await.expect("装配 CLI 环境失败");
    let provider: Arc<dyn TTSProvider> = fake;
    ctx.provide_named("fake-tts", provider).await.unwrap();
    ctx.set_default::<Arc<dyn TTSProvider>>("fake-tts").await.unwrap();
    (ctx, manager)
}

#[tokio::test]
async fn test_cli_list_shows_imported_and_synthesis_is_written_back() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use leo_core::plugin::StorageProvider;
    use leo_plugin_cli::{SynthesisOptions, execute_cli_with, list_projects, synthesize_chapter};

    let temp_dir = tempfile::tempdir().unwrap();
    let data_dir = temp_dir.path().join("data").to_str().unwrap().to_string();
    let novel = temp_dir.path().join("仙途.txt");
    std::fs::write(&novel, "第一章 开端\n他说：“你好。”\n风起了。\n").unwrap();
    let fake = Arc::new(fake_tts::FakeTts {
        calls: AtomicUsize::new(0),
    });

    // import 之后，list 必须能看到导入的项目（此前 list 读的是从未写入的 projects.csv）
    let (ctx, manager) = runtime_with_fake_tts(&data_dir, fake.clone()).await;
    assert!(list_projects(&ctx).await.unwrap().is_empty());
    execute_cli_with(
        Cli {
            command: Commands::Import {
                file: novel.to_str().unwrap().to_string(),
                title: Some("仙途, 第一部".to_string()),
            },
            data_dir: data_dir.clone(),
        },
        &ctx,
        &manager,
    )
    .await
    .expect("导入失败");
    let projects = list_projects(&ctx).await.unwrap();
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0].title, "仙途, 第一部", "书名含逗号也不能错列");
    assert_eq!(projects[0].chapter_count, 1);
    let project_id = projects[0].id.clone();
    execute_cli_with(
        Cli {
            command: Commands::List,
            data_dir: data_dir.clone(),
        },
        &ctx,
        &manager,
    )
    .await
    .expect("list 失败");

    // 合成后，切片引用写回剧本
    let options = SynthesisOptions {
        voice: "voice-a".to_string(),
        speed: 1.0,
        pitch: 0.0,
        force: false,
    };
    let report = synthesize_chapter(&ctx, &project_id, 0, &options).await.unwrap();
    assert!(report.synthesized >= 2, "{report:?}");
    assert_eq!(report.reused, 0);
    assert_eq!(report.total_duration_ms, 240 * report.synthesized as u64);
    let first_calls = fake.calls.load(Ordering::SeqCst);
    assert_eq!(first_calls, report.synthesized);
    drop(ctx);

    // 新进程（重新装配）重新加载剧本：每个片段都挂着音频引用，重跑同参数不会再次合成
    let (ctx, _manager) = runtime_with_fake_tts(&data_dir, fake.clone()).await;
    let storage = ctx.inject::<Arc<dyn StorageProvider>>().await.unwrap();
    let doc = storage.load_script(&project_id).await.unwrap().unwrap();
    let segments: Vec<_> = doc.chapters[0]
        .paragraphs
        .iter()
        .flat_map(|p| &p.sentences)
        .flat_map(|s| &s.segments)
        .filter(|seg| !seg.text.trim().is_empty())
        .collect();
    assert_eq!(segments.len(), report.synthesized);
    for seg in &segments {
        let audio = seg.audio.as_ref().expect("合成结果应写回片段");
        assert_eq!(audio.voice_id, "voice-a");
        assert_eq!(audio.tts_engine, "fake-tts");
        assert_eq!(audio.duration_ms, 240);
        assert!(storage.load_audio_clip(&audio.storage_key).await.unwrap().is_some());
    }

    let again = synthesize_chapter(&ctx, &project_id, 0, &options).await.unwrap();
    assert_eq!(again.synthesized, 0);
    assert_eq!(again.reused, report.synthesized);
    assert_eq!(fake.calls.load(Ordering::SeqCst), first_calls);

    // 换声线或 --force 会重新合成
    let other = SynthesisOptions {
        voice: "voice-b".to_string(),
        ..options.clone()
    };
    let switched = synthesize_chapter(&ctx, &project_id, 0, &other).await.unwrap();
    assert_eq!(switched.synthesized, report.synthesized);
    let forced = synthesize_chapter(&ctx, &project_id, 0, &SynthesisOptions { force: true, ..other })
        .await
        .unwrap();
    assert_eq!(forced.synthesized, report.synthesized);
}
