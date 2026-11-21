use std::fs;
use std::path::Path;
use clap::{Parser, Subcommand};
use leo_core::ast::SegmentKind;
use leo_core::spi::SynthesisPayload;
use leo_plugin_exchange::ExchangeHub;
use leo_plugin_storage::StorageHub;
use leo_plugin_tts::TtsHub;

use crate::setup_cli_runtime;

#[derive(Parser, Debug)]
#[command(
    name = "leo-cli",
    version = "0.2.0",
    about = "🎙️ Leo Studio 命令行有声书生产工具 (微内核插件化驱动)"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    #[arg(short, long, default_value = ".leo-studio", help = "数据存储目录")]
    pub data_dir: String,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    #[command(about = "导入小说纯文本，清洗并拆分为剧本 AST 持久化")]
    Import {
        #[arg(help = "文本小说文件路径，如: 测试.txt")]
        file: String,

        #[arg(short, long, help = "自定义项目标题（默认自动识别）")]
        title: Option<String>,
    },

    #[command(about = "列出当前已存储的剧目列表")]
    List,

    #[command(about = "导出指定剧本为制作台账(CSV)或完整 AST(JSON)")]
    Export {
        #[arg(help = "项目 ID")]
        project_id: String,

        #[arg(short, long, default_value = "csv", help = "导出格式: csv 或 json")]
        format: String,

        #[arg(short, long, help = "输出文件保存路径（默认自动命名）")]
        output: Option<String>,
    },

    #[command(about = "查看系统当前已激活加载的所有插件")]
    Plugins,

    #[command(about = "查询支持的 TTS 高保真语音合成声线")]
    Voices,

    #[command(about = "对指定章节的所有对白与旁白片段执行语音合成")]
    Synthesize {
        #[arg(help = "项目 ID")]
        project_id: String,

        #[arg(short, long, default_value_t = 0, help = "目标章节索引（从 0 开始）")]
        chapter: usize,

        #[arg(short, long, default_value = "zh-CN-YunxiNeural", help = "TTS 目标声线 ID")]
        voice: String,

        #[arg(short, long, default_value_t = 1.0, help = "语速倍率 (0.5 ~ 2.0)")]
        speed: f32,

        #[arg(short, long, default_value_t = 0.0, help = "音调调整 (-100% ~ +100%)")]
        pitch: f32,
    },
}

pub async fn execute_cli(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    let (ctx, manager) = setup_cli_runtime(&cli.data_dir).await?;

    match cli.command {
        Commands::Import { file, title } => {
            println!("📥 正在读取文件: {file} ...");
            let bytes = fs::read(&file).map_err(|e| format!("读取文件失败 [{file}]: {e}"))?;
            let filename = Path::new(&file)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("novel.txt");

            println!("⚙️ 正在执行文本清洗、章节拆分与台词提取 (AST 生成) ...");
            let (_, importer) = ExchangeHub::of(&ctx).await.importer_for(filename)?;
            let mut doc = importer.import(filename, &bytes).await?;

            if let Some(custom_title) = title {
                doc.title = custom_title;
            }

            // 持久化保存
            if let Ok(storage) = StorageHub::of(&ctx).await.default_storage() {
                storage.save_script(&doc).await?;
            }

            let mut total_sentences = 0;
            let mut total_segments = 0;
            let mut total_words = 0;
            for ch in &doc.chapters {
                for p in &ch.paragraphs {
                    for s in &p.sentences {
                        total_sentences += 1;
                        total_segments += s.segments.len();
                        total_words += s.word_count;
                    }
                }
            }

            println!("\n✅ 小说剧本导入成功！");
            println!("─────────────────────────────────────────");
            println!("  项目 ID     : {}", doc.id);
            println!("  书名标题   : {}", doc.title);
            println!("  章节总数   : {} 章", doc.chapters.len());
            println!("  句子总数   : {} 句", total_sentences);
            println!("  台词片段数 : {} 段", total_segments);
            println!("  总字数统计 : {} 字", total_words);
            println!("─────────────────────────────────────────");
            println!("提示: 可运行 `leo-cli export {} --format csv` 导出配音台账。", doc.id);
        }

        Commands::List => {
            let projects_file = Path::new(&cli.data_dir).join("projects.csv");
            if !projects_file.exists() {
                println!("📂 当前存储库尚未创建任何项目。");
                return Ok(());
            }

            let content = fs::read_to_string(&projects_file)?;
            println!("\n📚 Leo Studio 剧目列表 ({})：", projects_file.display());
            println!("────────────────────────────────────────────────────────────────────────────────────────");
            println!("{:<34} {:<20} {:<10} {:<10}", "项目 ID", "名称", "章节数", "状态");
            println!("────────────────────────────────────────────────────────────────────────────────────────");

            for (idx, line) in content.lines().enumerate() {
                if idx == 0 || line.trim().is_empty() {
                    continue;
                }
                let parts: Vec<&str> = line.split(',').collect();
                if parts.len() >= 5 {
                    let project_id = parts.first().unwrap_or(&"");
                    let name = parts.get(1).unwrap_or(&"");
                    let chapters = parts.get(5).unwrap_or(&"0");
                    let status = parts.get(4).unwrap_or(&"active");
                    println!("{:<34} {:<20} {:<10} {:<10}", project_id, name, chapters, status);
                }
            }
            println!("────────────────────────────────────────────────────────────────────────────────────────");
        }

        Commands::Export {
            project_id,
            format,
            output,
        } => {
            println!("📤 正在加载项目 {project_id} ...");
            let storage = StorageHub::of(&ctx)
                .await
                .default_storage()
                .map_err(|e| format!("未挂载存储插件: {e}"))?;

            let doc = storage
                .load_script(&project_id)
                .await?
                .ok_or_else(|| format!("未在存储库中找到项目 ID: {project_id}"))?;

            let (_, exporter) = ExchangeHub::of(&ctx).await.exporter(Some(&format))?;
            let bytes = exporter.export(&doc).await?;
            let ext = exporter.file_extension().to_string();

            let target_path = output.unwrap_or_else(|| format!("{}_{}.{}", doc.title, doc.id, ext));
            fs::write(&target_path, &bytes)?;
            println!("✅ 剧本已成功导出！");
            println!("  格式: {}", ext.to_uppercase());
            println!("  文件: {}", Path::new(&target_path).canonicalize().unwrap_or(target_path.into()).display());
            println!("  大小: {} 字节", bytes.len());
        }

        Commands::Plugins => {
            let plugins = manager.statuses().await;
            println!("\n🧩 当前系统已登记的插件列表 (共 {} 个)：", plugins.len());
            println!("───────────────────────────────────────────────────────────────────────────────────");
            println!("{:<32} {:<12} {:<10} {:<8} {:<30}", "插件 ID", "分类", "版本", "状态", "功能特性");
            println!("───────────────────────────────────────────────────────────────────────────────────");
            for p in plugins {
                let cat = format!("{:?}", p.manifest.category).to_lowercase();
                let state = format!("{:?}", p.state).to_lowercase();
                let caps = p.manifest.capabilities.join(", ");
                println!("{:<32} {:<12} {:<10} {:<8} {:<30}", p.manifest.id, cat, p.manifest.version, state, caps);
            }
            println!("───────────────────────────────────────────────────────────────────────────────────");
        }

        Commands::Voices => {
            println!("🎙️ 正在查询可用 TTS 声线库 ...");
            let voices = TtsHub::of(&ctx).await.list_voices().await?;
            println!("\n📢 支持的 TTS 语音合成声线 (共 {} 种)：", voices.len());
            println!("──────────────────────────────────────────────────────────────────────────");
            println!("{:<24} {:<12} {:<8} {:<10} {:<20}", "声线 ID", "显示名称", "性别", "语言", "描述");
            println!("──────────────────────────────────────────────────────────────────────────");
            for v in voices {
                println!("{:<24} {:<12} {:<8} {:<10} {:<20}", v.id, v.info.name, v.info.gender, v.info.locale, v.info.description);
            }
            println!("──────────────────────────────────────────────────────────────────────────");
        }

        Commands::Synthesize {
            project_id,
            chapter,
            voice,
            speed,
            pitch,
        } => {
            println!("🎙️ 准备合成项目 [{project_id}] 的第 {chapter} 章音频 ...");
            let storage = StorageHub::of(&ctx)
                .await
                .default_storage()
                .map_err(|e| format!("未挂载存储插件: {e}"))?;
            let tts = TtsHub::of(&ctx).await;
            tts.route(&voice).map_err(|e| format!("未挂载 TTS 插件: {e}"))?;

            let doc = storage
                .load_script(&project_id)
                .await?
                .ok_or_else(|| format!("未找到项目 ID: {project_id}"))?;

            let target_chapter = doc
                .chapters
                .get(chapter)
                .ok_or_else(|| format!("项目仅有 {} 章，索引 {} 越界", doc.chapters.len(), chapter))?;

            println!("📖 正在处理章节: 《{}》", target_chapter.title);

            let mut clips_count = 0;
            let mut total_duration_ms = 0u64;

            for (p_idx, para) in target_chapter.paragraphs.iter().enumerate() {
                for (s_idx, sentence) in para.sentences.iter().enumerate() {
                    for (seg_idx, seg) in sentence.segments.iter().enumerate() {
                        let text = seg.text.trim();
                        if text.is_empty() {
                            continue;
                        }

                        let role_tag = match seg.kind {
                            SegmentKind::Dialogue => "【台词对白】",
                            SegmentKind::Narration => "【旁白叙述】",
                            SegmentKind::Sfx => "【音效标记】",
                        };

                        let payload = SynthesisPayload {
                            text: text.to_string(),
                            voice: voice.clone(),
                            speed,
                            pitch,
                            emotion: None,
                        };

                        print!("  -> 段落[{p_idx}]-句[{s_idx}]-切片[{seg_idx}] {role_tag} \"{text}\" 合成中... ");
                        let out = tts.synthesize(&payload).await?;
                        let clip_id = uuid::Uuid::new_v4().simple().to_string();
                        let clip_ref = storage
                            .save_audio_clip(&clip_id, &out.audio_bytes, out.format)
                            .await?;

                        clips_count += 1;
                        total_duration_ms += out.duration_ms;
                        println!("完成! [{:.2}s, 存储键: {}]", out.duration_ms as f64 / 1000.0, clip_ref.storage_key);
                    }
                }
            }

            println!("\n🎉 章节语音合成全量完成！");
            println!("  生成切片数   : {} 个", clips_count);
            println!("  累计音频时长 : {:.2} 秒", total_duration_ms as f64 / 1000.0);
            println!("  采用声线     : {}", voice);
        }
    }

    Ok(())
}
