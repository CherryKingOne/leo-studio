use std::fs;
use std::path::Path;
use std::sync::Arc;

use clap::{Parser, Subcommand};
use chrono::Utc;
use leo_core::ast::{AudioClipRef, AudioFormat, SegmentKind, mp3_duration_ms, synthesis_clip_id};
use leo_core::context::Context;
use leo_core::plugin::{
    Exporter, Importer, PluginManager, ScriptSummary, StorageProvider, SynthesisPayload,
    TTSProvider,
};

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

        #[arg(short, long, default_value_t = 0.0, allow_hyphen_values = true, help = "音调偏移，单位 Hz (-100 ~ +100，0 为原调)")]
        pitch: f32,

        #[arg(long, help = "忽略已有切片，全部重新合成")]
        force: bool,
    },
}

pub async fn execute_cli(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    let (ctx, manager) = setup_cli_runtime(&cli.data_dir).await?;
    execute_cli_with(cli, &ctx, &manager).await
}

/// 在已装配好的运行时上执行命令（便于测试或宿主替换插件后复用）
pub async fn execute_cli_with(
    cli: Cli,
    ctx: &Context,
    manager: &PluginManager,
) -> Result<(), Box<dyn std::error::Error>> {
    let ctx = ctx.clone();
    match cli.command {
        Commands::Import { file, title } => {
            println!("📥 正在读取文件: {file} ...");
            let bytes = fs::read(&file).map_err(|e| format!("读取文件失败 [{file}]: {e}"))?;
            let filename = Path::new(&file)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("novel.txt");

            println!("⚙️ 正在执行文本清洗、章节拆分与台词提取 (AST 生成) ...");
            let importer = find_importer(&ctx, filename).await?;
            let mut doc = importer.import(filename, &bytes).await?;

            if let Some(custom_title) = title {
                doc.title = custom_title;
            }

            // 持久化保存：没有存储插件时直接报错，避免“导入成功”却什么都没保存
            let storage = ctx
                .inject::<Arc<dyn StorageProvider>>()
                .await
                .map_err(|_| "未挂载 StorageProvider 存储插件，无法保存导入结果")?;
            storage.save_script(&doc).await?;

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
            let projects = list_projects(&ctx).await?;
            if projects.is_empty() {
                println!("📂 当前存储库尚未创建任何项目。");
                return Ok(());
            }

            println!("\n📚 Leo Studio 剧目列表 ({})：", cli.data_dir);
            println!("────────────────────────────────────────────────────────────────────────────────────────");
            println!("{:<34} {:<20} {:<10} {:<10}", "项目 ID", "名称", "章节数", "总字数");
            println!("────────────────────────────────────────────────────────────────────────────────────────");
            for project in projects {
                println!(
                    "{:<34} {:<20} {:<10} {:<10}",
                    project.id, project.title, project.chapter_count, project.total_words
                );
            }
            println!("────────────────────────────────────────────────────────────────────────────────────────");
        }

        Commands::Export {
            project_id,
            format,
            output,
        } => {
            println!("📤 正在加载项目 {project_id} ...");
            let storage = ctx
                .inject::<Arc<dyn StorageProvider>>()
                .await
                .map_err(|_| "未挂载 StorageProvider 存储插件")?;

            let doc = storage
                .load_script(&project_id)
                .await?
                .ok_or_else(|| format!("未在存储库中找到项目 ID: {project_id}"))?;

            let format = format.to_lowercase();
            let Ok(exporter) = ctx.inject_named::<Arc<dyn Exporter>>(&format).await else {
                let available: Vec<String> = ctx
                    .inject_all::<Arc<dyn Exporter>>()
                    .await
                    .into_iter()
                    .map(|(name, _)| name)
                    .collect();
                return Err(
                    format!("不支持的导出格式 {format}，可用格式：{}", available.join(", ")).into(),
                );
            };
            let bytes = exporter.export(&doc).await?;
            let ext = exporter.format_name().to_string();

            let target_path = output.unwrap_or_else(|| format!("{}_{}.{}", doc.title, doc.id, ext));
            fs::write(&target_path, &bytes)?;
            println!("✅ 剧本已成功导出！");
            println!("  格式: {}", ext.to_uppercase());
            println!("  文件: {}", Path::new(&target_path).canonicalize().unwrap_or(target_path.into()).display());
            println!("  大小: {} 字节", bytes.len());
        }

        Commands::Plugins => {
            let plugins = manager.list_manifests().await;
            println!("\n🧩 当前系统已激活挂载的插件列表 (共 {} 个)：", plugins.len());
            println!("───────────────────────────────────────────────────────────────────────────────────");
            println!("{:<32} {:<12} {:<10} {:<30}", "插件 ID", "分类", "版本", "功能特性");
            println!("───────────────────────────────────────────────────────────────────────────────────");
            for p in plugins {
                let cat = format!("{:?}", p.category).to_lowercase();
                let caps = p.capabilities.join(", ");
                println!("{:<32} {:<12} {:<10} {:<30}", p.id, cat, p.version, caps);
            }
            println!("───────────────────────────────────────────────────────────────────────────────────");
        }

        Commands::Voices => {
            println!("🎙️ 正在查询可用 TTS 声线库 ...");
            let tts = ctx
                .inject::<Arc<dyn TTSProvider>>()
                .await
                .map_err(|_| "未挂载 TTS 语音合成插件")?;

            let voices = tts.list_voices().await?;
            println!("\n📢 支持的 TTS 语音合成声线 (共 {} 种)：", voices.len());
            println!("──────────────────────────────────────────────────────────────────────────");
            println!("{:<24} {:<12} {:<8} {:<10} {:<20}", "声线 ID", "显示名称", "性别", "语言", "描述");
            println!("──────────────────────────────────────────────────────────────────────────");
            for v in voices {
                println!("{:<24} {:<12} {:<8} {:<10} {:<20}", v.id, v.name, v.gender, v.locale, v.description);
            }
            println!("──────────────────────────────────────────────────────────────────────────");
        }

        Commands::Synthesize {
            project_id,
            chapter,
            voice,
            speed,
            pitch,
            force,
        } => {
            println!("🎙️ 准备合成项目 [{project_id}] 的第 {chapter} 章音频 ...");
            let report = synthesize_chapter(
                &ctx,
                &project_id,
                chapter,
                &SynthesisOptions {
                    voice: voice.clone(),
                    speed,
                    pitch,
                    force,
                },
            )
            .await?;

            println!("\n🎉 章节语音合成全量完成！");
            println!("  新合成切片数 : {} 个", report.synthesized);
            println!("  复用已有切片 : {} 个", report.reused);
            println!("  累计音频时长 : {:.2} 秒", report.total_duration_ms as f64 / 1000.0);
            println!("  采用声线     : {}", voice);
            println!("  切片引用已写回剧本，可通过 export --format json 查看。");
        }
    }

    Ok(())
}

/// 按文件扩展名挑选第一个声明支持该扩展名的 Importer 插件；无扩展名时使用默认 Importer
async fn find_importer(
    ctx: &Context,
    filename: &str,
) -> Result<Arc<dyn Importer>, Box<dyn std::error::Error>> {
    let extension = Path::new(filename)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(str::to_lowercase)
        .unwrap_or_default();
    if extension.is_empty() {
        return Ok(ctx.inject::<Arc<dyn Importer>>().await?.as_ref().clone());
    }
    ctx.inject_all::<Arc<dyn Importer>>()
        .await
        .into_iter()
        .map(|(_, importer)| importer.as_ref().clone())
        .find(|importer| {
            importer
                .supported_extensions()
                .iter()
                .any(|ext| ext.eq_ignore_ascii_case(&extension))
        })
        .ok_or_else(|| format!("没有可处理 .{extension} 文件的导入插件").into())
}

/// 通过 StorageProvider 列出已存储的剧目（与 `import` 写入的是同一份数据）
pub async fn list_projects(ctx: &Context) -> Result<Vec<ScriptSummary>, Box<dyn std::error::Error>> {
    let storage = ctx
        .inject::<Arc<dyn StorageProvider>>()
        .await
        .map_err(|_| "未挂载 StorageProvider 存储插件")?;
    Ok(storage.list_scripts().await?)
}

/// 章节合成参数
#[derive(Debug, Clone)]
pub struct SynthesisOptions {
    pub voice: String,
    pub speed: f32,
    pub pitch: f32,
    /// 为 true 时忽略已有切片，全部重新合成
    pub force: bool,
}

/// 章节合成结果统计
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SynthesisReport {
    pub synthesized: usize,
    pub reused: usize,
    pub total_duration_ms: u64,
}

/// 合成指定章节的全部片段，并把生成的 AudioClipRef 写回剧本后保存。
///
/// 切片 ID 与 REST/MCP 相同，由片段 ID + TTS 引擎 + 声线 + 语速 + 音调 + 文本计算；
/// 片段已挂载同一切片且音频文件仍在时直接复用（`force` 为 true 时除外）。
/// 中途合成失败时，已完成的切片引用仍会先保存再返回错误。
pub async fn synthesize_chapter(
    ctx: &Context,
    project_id: &str,
    chapter: usize,
    options: &SynthesisOptions,
) -> Result<SynthesisReport, Box<dyn std::error::Error>> {
    let storage = ctx
        .inject::<Arc<dyn StorageProvider>>()
        .await
        .map_err(|_| "未挂载 StorageProvider 插件")?;
    let tts = ctx
        .inject::<Arc<dyn TTSProvider>>()
        .await
        .map_err(|_| "未挂载 TTSProvider 插件")?;

    let mut doc = storage
        .load_script(project_id)
        .await?
        .ok_or_else(|| format!("未找到项目 ID: {project_id}"))?;
    let chapter_count = doc.chapters.len();
    let target_chapter = doc
        .chapters
        .get_mut(chapter)
        .ok_or_else(|| format!("项目仅有 {chapter_count} 章，索引 {chapter} 越界"))?;

    println!("📖 正在处理章节: 《{}》", target_chapter.title);

    let engine = tts.provider_id().to_string();
    let mut report = SynthesisReport::default();
    let mut failure: Option<Box<dyn std::error::Error>> = None;

    'outer: for (p_idx, para) in target_chapter.paragraphs.iter_mut().enumerate() {
        for (s_idx, sentence) in para.sentences.iter_mut().enumerate() {
            for (seg_idx, seg) in sentence.segments.iter_mut().enumerate() {
                let text = seg.text.trim().to_string();
                if text.is_empty() {
                    continue;
                }

                let clip_id = synthesis_clip_id(
                    &seg.id,
                    &engine,
                    &options.voice,
                    options.speed,
                    options.pitch,
                    &text,
                );

                if !options.force
                    && let Some(existing) = seg.audio.as_ref()
                    && existing.clip_id == clip_id
                    && storage
                        .load_audio_clip(&existing.storage_key)
                        .await
                        .ok()
                        .flatten()
                        .is_some()
                {
                    report.reused += 1;
                    report.total_duration_ms += existing.duration_ms;
                    continue;
                }

                let role_tag = match seg.kind {
                    SegmentKind::Dialogue => "【台词对白】",
                    SegmentKind::Narration => "【旁白叙述】",
                    SegmentKind::Sfx => "【音效标记】",
                };
                print!("  -> 段落[{p_idx}]-句[{s_idx}]-切片[{seg_idx}] {role_tag} \"{text}\" 合成中... ");

                let payload = SynthesisPayload {
                    text: text.clone(),
                    voice: options.voice.clone(),
                    speed: options.speed,
                    pitch: options.pitch,
                    emotion: None,
                };
                let out = match tts.synthesize(&payload).await {
                    Ok(out) => out,
                    Err(err) => {
                        failure = Some(Box::new(err));
                        break 'outer;
                    }
                };
                let clip_ref = match storage
                    .save_audio_clip(&clip_id, &out.audio_bytes, out.format)
                    .await
                {
                    Ok(clip_ref) => clip_ref,
                    Err(err) => {
                        failure = Some(Box::new(err));
                        break 'outer;
                    }
                };
                let duration_ms = match out.format {
                    AudioFormat::Mp3 => mp3_duration_ms(&out.audio_bytes).unwrap_or(out.duration_ms),
                    _ => out.duration_ms,
                };

                seg.audio = Some(AudioClipRef {
                    clip_id: clip_ref.clip_id,
                    format: out.format,
                    sample_rate: out.sample_rate,
                    duration_ms,
                    storage_key: clip_ref.storage_key.clone(),
                    tts_engine: engine.clone(),
                    voice_id: options.voice.clone(),
                    created_at: Utc::now(),
                });
                seg.is_dirty = false;

                report.synthesized += 1;
                report.total_duration_ms += duration_ms;
                println!("完成! [{:.2}s, 存储键: {}]", duration_ms as f64 / 1000.0, clip_ref.storage_key);
            }
        }
    }

    // 把切片引用写回剧本：即使中途失败，也保存已完成的部分，避免音频文件成为孤儿
    if report.synthesized > 0 {
        doc.updated_at = Utc::now();
        storage.save_script(&doc).await?;
    }

    match failure {
        Some(err) => Err(err),
        None => Ok(report),
    }
}
