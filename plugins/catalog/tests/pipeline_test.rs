use std::sync::Arc;

use leo_core::ast::{AudioClipRef, AudioFormat, ChapterNode, ParagraphNode, SegmentKind, SegmentNode, SentenceNode};
use leo_core::context::Context;
use leo_core::pipeline::DirtyTracker;
use leo_core::plugin::{
    ChapterSplitter, DialogueSplitter, PluginManager, SynthesisPayload, TTSProvider, TextCleaner,
};
use leo_plugin_cleaner_kaomoji::KaomojiCleanerPlugin;
use leo_plugin_dialogue_quotes::DialogueQuotesPlugin;
use leo_plugin_parser_novel_regex::NovelRegexParserPlugin;
use leo_plugin_tts_edge::EdgeTtsPlugin;

#[tokio::test]
async fn test_end_to_end_audiobook_pipeline() {
    // 1. 初始化 Cordis 风格上下文容器与插件管理器
    let ctx = Context::new("novel_production_p1");
    let manager = PluginManager::new();

    // 2. 注册并装配官方插件四大金刚
    manager.register(Arc::new(KaomojiCleanerPlugin::new())).await.unwrap();
    manager.register(Arc::new(NovelRegexParserPlugin::new())).await.unwrap();
    manager.register(Arc::new(DialogueQuotesPlugin::new())).await.unwrap();
    manager.register(Arc::new(EdgeTtsPlugin::new())).await.unwrap();

    // 一键挂载所有插件
    manager.apply_all(&ctx).await.expect("插件批量挂载失败");

    // 依赖注入按需提取四大服务
    let cleaner = ctx.registry::<dyn TextCleaner>().default_value().unwrap();
    let chapter_splitter = ctx.registry::<dyn ChapterSplitter>().default_value().unwrap();
    let dialogue_splitter = ctx.registry::<dyn DialogueSplitter>().default_value().unwrap();
    let tts = ctx.registry::<dyn TTSProvider>().default_value().unwrap();

    // 3. 原始网络小说输入（带颜文字、未清洗符号、旁白与双引号台词）
    let raw_novel_text = r#"
第一章 宗门风云
林冲(╯°□°)╯冷笑一声：“师姐，你若再逼我，休怪我手中宝刀无情！”
师姐却微微一笑：“小师弟，你命脉皆系于我手，拿什么与我斗？”
"#;

    // Step 1: 机械与颜文字清洗
    let cleaned_lines: Vec<String> = raw_novel_text
        .lines()
        .map(|line| cleaner.clean_line(line))
        .filter(|line| !line.is_empty())
        .collect();
    let cleaned_text = cleaned_lines.join("\n");
    assert!(!cleaned_text.contains("(╯°□°)╯"), "颜文字必须被清洗剥离");

    // Step 2: 章节识别与指纹提取
    let raw_chapters = chapter_splitter.split_chapters(&cleaned_text);
    assert_eq!(raw_chapters.len(), 1);
    let raw_chapter = &raw_chapters[0];
    assert_eq!(raw_chapter.title, "第一章 宗门风云");

    let fingerprint = chapter_splitter.resolve_fingerprint(&raw_chapter.title);
    assert_eq!(fingerprint, "ch_1");

    // 构造 AST 章节节点
    let mut chapter_ast = ChapterNode::new("ch_uuid_001".into(), 1, fingerprint, raw_chapter.title.clone());

    // Step 3: 单句与台词拆解（构建 AST 子树）
    let mut paragraph_ast = ParagraphNode {
        id: "p_uuid_001".into(),
        index: 0,
        sentences: Vec::new(),
    };

    let mut segment_global_index = 0;
    for (s_idx, line) in raw_chapter.content.lines().filter(|l| !l.trim().is_empty()).enumerate() {
        let mut sentence_ast = SentenceNode::new(format!("s_{s_idx}"), s_idx, line.to_string());
        let raw_segments = dialogue_splitter.split_sentence(line);

        for raw_seg in raw_segments {
            let mut seg_node = SegmentNode::new(
                format!("seg_{segment_global_index}"),
                segment_global_index,
                raw_seg.kind,
                raw_seg.text,
            );

            // Step 4: 角色与声线绑定（选角系统逻辑）
            match seg_node.kind {
                SegmentKind::Narration => {
                    seg_node.speaker_id = Some("narrator".into()); // 旁白用沉稳大叔音
                }
                SegmentKind::Dialogue => {
                    if seg_node.text.contains("宝刀无情") {
                        seg_node.speaker_id = Some("lin_chong".into()); // 男主用热血青年音
                    } else {
                        seg_node.speaker_id = Some("senior_sister".into()); // 师姐用灵动知性音
                    }
                }
                SegmentKind::Sfx => {}
            }

            sentence_ast.segments.push(seg_node);
            segment_global_index += 1;
        }
        paragraph_ast.sentences.push(sentence_ast);
    }
    chapter_ast.paragraphs.push(paragraph_ast);
    chapter_ast.recalculate_word_count();

    // 验证 AST 拆解出的片段数量：
    // 第1句：旁白("林冲冷笑一声：") + 台词("“师姐，你若再逼我，休怪我手中宝刀无情！”")
    // 第2句：旁白("师姐却微微一笑：") + 台词("“小师弟，你命脉皆系于我手，拿什么与我斗？”")
    // 共计 4 个片段
    let dirty_summary = DirtyTracker::inspect_chapter(&chapter_ast);
    assert_eq!(dirty_summary.total_segments, 4);
    assert_eq!(dirty_summary.dirty_segments, 4);
    assert!(!dirty_summary.is_fully_clean);

    // Step 5: 真实 TTS 语音渲染流（仅渲染第一句台词做连通性验证）
    let dirty_segments = DirtyTracker::collect_dirty_segments(&mut chapter_ast);
    assert_eq!(dirty_segments.len(), 4);

    let first_dialogue = dirty_segments.into_iter().find(|s| s.kind == SegmentKind::Dialogue).unwrap();
    println!("准备合成第一句台词：{}", first_dialogue.text);

    let payload = SynthesisPayload {
        text: first_dialogue.text.clone(),
        voice: "zh-CN-YunxiNeural".into(),
        speed: 1.0,
        pitch: 0.0,
        emotion: None,
    };

    if let Ok(output) = tts.synthesize(&payload).await {
        println!("第一句台词合成成功，输出 {} 字节 MP3 音频！", output.audio_bytes.len());
        assert!(!output.audio_bytes.is_empty());

        // 挂载生成的 AudioClipRef 回 AST 节点，消除脏标记
        first_dialogue.audio = Some(AudioClipRef {
            clip_id: "clip_001".into(),
            format: AudioFormat::Mp3,
            sample_rate: output.sample_rate,
            duration_ms: output.duration_ms,
            storage_key: "audio/clips/clip_001.mp3".into(),
            tts_engine: "edge-tts".into(),
            voice_id: "zh-CN-YunxiNeural".into(),
            created_at: chrono::Utc::now(),
        });
        first_dialogue.is_dirty = false;
    }

    // Step 6: 检查局部失效追踪状态更新
    let updated_summary = DirtyTracker::inspect_chapter(&chapter_ast);
    assert_eq!(updated_summary.total_segments, 4);
    // 已经有 1 个切片成功合成并被挂载，剩余 3 个待合成
    assert_eq!(updated_summary.clean_segments, 1);
    assert_eq!(updated_summary.dirty_segments, 3);
}

/// 通过插件目录装配全部插件后，各注册层都能拿到驱动
#[tokio::test]
async fn test_catalog_bootstrap_wires_every_registry() {
    use leo_plugin_catalog::{ExchangeHub, HostEnv, LlmHub, PluginsConfig, StorageHub, TextHub, TtsHub, bootstrap};
    let dir = std::env::temp_dir().join(format!("leo-catalog-{}", std::process::id()));
    let env = HostEnv { csv_dir: dir.clone() };
    let (ctx, manager) = bootstrap("catalog", &env, &PluginsConfig::default()).await.unwrap();

    assert_eq!(manager.list_manifests().await.len(), leo_plugin_catalog::CATALOG.len());
    let exchange = ExchangeHub::of(&ctx).await;
    assert_eq!(exchange.importer_for("书.TXT").unwrap().0, "txt");
    assert_eq!(exchange.exporter(Some("csv")).unwrap().0, "csv");
    assert_eq!(exchange.exporter(None).unwrap().0, "json");
    assert!(StorageHub::of(&ctx).await.default_storage().is_ok());
    assert_eq!(TtsHub::of(&ctx).await.route("zh-CN-YunxiNeural").unwrap().provider_id, "edge");
    assert_eq!(TextHub::of(&ctx).await.clean_line("  "), "");
    assert_eq!(LlmHub::of(&ctx).await.provider(None).unwrap().0, "deepseek");

    // 端到端：导入 → 存储 → 读回 → 导出
    let (_, importer) = exchange.importer_for("测试——作者——玄幻.txt").unwrap();
    let doc = importer.import("测试——作者——玄幻.txt", "第一章 开端\n“你好。”他说。".as_bytes()).await.unwrap();
    let storage = StorageHub::of(&ctx).await.default_storage().unwrap();
    storage.save_script(&doc).await.unwrap();
    let loaded = storage.load_script(&doc.id).await.unwrap().expect("应能读回");
    let bytes = exchange.exporter(Some("json")).unwrap().1.export(&loaded).await.unwrap();
    assert!(!bytes.is_empty());

    // 卸载 csv 导出器后 csv 格式不可用，json 仍可用（换插件不动接口层）
    manager.unload("leo-plugin-exporter-csv").await.unwrap();
    assert!(exchange.exporter(Some("csv")).is_err());
    assert!(exchange.exporter(Some("json")).is_ok());
    let _ = std::fs::remove_dir_all(dir);
}

/// 配置的默认 LLM 驱动被停用时，调用直接报错而不是换别的驱动
#[tokio::test]
async fn test_catalog_llm_default_disabled_is_error() {
    use leo_plugin_catalog::{HostEnv, LlmHub, PluginsConfig, bootstrap};
    let env = HostEnv { csv_dir: std::env::temp_dir().join(format!("leo-catalog-llm-{}", std::process::id())) };
    let config: PluginsConfig =
        serde_json::from_str(r#"{"disabled":["leo-plugin-llm-deepseek"],"llm":{"default":"deepseek"}}"#).unwrap();
    let (ctx, _) = bootstrap("catalog", &env, &config).await.unwrap();
    let err = LlmHub::of(&ctx).await.provider(None).err().expect("应报错").to_string();
    assert!(err.contains("deepseek"), "{err}");
    let _ = std::fs::remove_dir_all(env.csv_dir);
}

/// 停用插件只需改配置
#[tokio::test]
async fn test_catalog_disabled_plugins() {
    use leo_plugin_catalog::{ExchangeHub, HostEnv, PluginsConfig, bootstrap};
    let env = HostEnv { csv_dir: std::env::temp_dir().join(format!("leo-catalog-d-{}", std::process::id())) };
    let config: PluginsConfig = serde_json::from_str(r#"{"disabled":["leo-plugin-exporter-json"]}"#).unwrap();
    let (ctx, _) = bootstrap("catalog", &env, &config).await.unwrap();
    let exchange = ExchangeHub::of(&ctx).await;
    assert!(exchange.exporter(Some("json")).is_err());
    assert_eq!(exchange.exporter(None).unwrap().0, "csv");
    let _ = std::fs::remove_dir_all(env.csv_dir);
}
