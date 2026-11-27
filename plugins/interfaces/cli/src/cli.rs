use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::commands;
use crate::runtime;

#[derive(Parser, Debug)]
#[command(
    name = "leo-cli",
    version = "0.2.0",
    about = "Leo Studio 项目管理与有声书生产命令行工具"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    #[arg(long, global = true, default_value = ".", help = "数据根目录")]
    pub data_dir: PathBuf,

    #[arg(long, global = true, help = "CSV 目录，默认 <data-dir>/.leo-studio")]
    pub csv_dir: Option<PathBuf>,
}

#[derive(Subcommand, Debug)]
#[command(rename_all = "kebab-case")]
pub enum Commands {
    #[command(about = "v3 项目管理")]
    Project {
        #[command(subcommand)]
        command: ProjectCommands,
    },

    #[command(about = "v3 回收站管理")]
    RecycleBin {
        #[command(subcommand)]
        command: RecycleBinCommands,
    },

    #[command(about = "导入小说纯文本到 AST 生产存储")]
    Import {
        #[arg(help = "文本小说文件路径")]
        file: PathBuf,
        #[arg(short, long, help = "自定义项目标题")]
        title: Option<String>,
    },

    #[command(about = "导出 AST 剧本为 CSV 或 JSON")]
    Export {
        #[arg(help = "项目 ID")]
        project_id: String,
        #[arg(short, long, default_value = "csv", help = "导出格式: csv 或 json")]
        format: String,
        #[arg(short, long, help = "输出文件路径")]
        output: Option<PathBuf>,
    },

    #[command(about = "查看已加载插件")]
    Plugins,

    #[command(about = "查询 TTS 声线")]
    Voices,

    #[command(about = "对 AST 剧本章节执行语音合成")]
    Synthesize {
        #[arg(help = "项目 ID")]
        project_id: String,
        #[arg(short, long, default_value_t = 0, help = "章节索引")]
        chapter: usize,
        #[arg(short, long, default_value = "zh-CN-YunxiNeural", help = "声线 ID")]
        voice: String,
        #[arg(short, long, default_value_t = 1.0, help = "语速倍率")]
        speed: f32,
        #[arg(short, long, default_value_t = 0.0, help = "音调调整")]
        pitch: f32,
    },
}

#[derive(Subcommand, Debug)]
#[command(rename_all = "kebab-case")]
pub enum ProjectCommands {
    #[command(about = "创建项目并上传小说")]
    Create {
        #[arg(long)]
        name: String,
        #[arg(long)]
        file: PathBuf,
    },
    #[command(about = "项目列表")]
    List {
        #[arg(long, default_value_t = 1)]
        page: usize,
        #[arg(long, default_value_t = 20)]
        page_size: usize,
        #[arg(long)]
        status: Option<String>,
        #[arg(long)]
        keyword: Option<String>,
        #[arg(long, default_value = "created_at")]
        sort: String,
        #[arg(long, default_value = "desc")]
        order: String,
    },
    #[command(about = "项目详情")]
    Get { project_id: String },
    #[command(about = "更新项目")]
    Update {
        project_id: String,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        status: Option<String>,
    },
    #[command(about = "软删除项目")]
    Delete {
        project_id: String,
        #[arg(long)]
        confirm: bool,
    },
    #[command(about = "项目章节管理")]
    Chapters {
        #[command(subcommand)]
        command: ChapterCommands,
    },
    #[command(about = "项目角色管理")]
    Characters {
        #[command(subcommand)]
        command: CharacterCommands,
    },
    #[command(about = "项目多音字管理")]
    Polyphones {
        #[command(subcommand)]
        command: PolyphoneCommands,
    },
}

#[derive(Subcommand, Debug)]
#[command(rename_all = "kebab-case")]
pub enum ChapterCommands {
    #[command(about = "章节大纲列表")]
    List {
        project_id: String,
        #[arg(long)]
        keyword: Option<String>,
    },
    #[command(about = "批量追加上传章节")]
    Append {
        project_id: String,
        #[arg(long)]
        file: PathBuf,
        #[arg(long)]
        overwrite_existing: bool,
        #[arg(long, default_value_t = true)]
        dry_run: bool,
        #[arg(long)]
        confirm: bool,
    },
    #[command(about = "章节详情")]
    Get {
        project_id: String,
        chapter_id: String,
    },
    #[command(about = "删除章节")]
    Delete {
        project_id: String,
        chapter_id: String,
        #[arg(long)]
        confirm: bool,
    },
}

#[derive(Subcommand, Debug)]
#[command(rename_all = "kebab-case")]
pub enum CharacterCommands {
    #[command(about = "新建角色档案")]
    Create {
        project_id: String,
        #[arg(long)]
        name: String,
        #[arg(long = "alias")]
        aliases: Vec<String>,
        #[arg(long)]
        gender: Option<String>,
        #[arg(long)]
        age: Option<String>,
        #[arg(long)]
        role_type: Option<String>,
        #[arg(long)]
        status: Option<String>,
        #[arg(long)]
        description: Option<String>,
        #[arg(long = "tag")]
        tags: Vec<String>,
    },
    #[command(about = "角色列表与过滤")]
    List {
        project_id: String,
        #[arg(long, default_value_t = 1)]
        page: usize,
        #[arg(long, default_value_t = 20)]
        page_size: usize,
        #[arg(long)]
        keyword: Option<String>,
        #[arg(long)]
        gender: Option<String>,
        #[arg(long)]
        role_type: Option<String>,
        #[arg(long, default_value = "active")]
        status: String,
        #[arg(long)]
        tag: Option<String>,
        #[arg(long, default_value = "created_at")]
        sort: String,
        #[arg(long, default_value = "desc")]
        order: String,
    },
    #[command(about = "角色详情")]
    Get {
        project_id: String,
        character_id: String,
    },
    #[command(about = "更新角色档案")]
    Update {
        project_id: String,
        character_id: String,
        #[arg(long)]
        name: Option<String>,
        #[arg(long = "alias")]
        aliases: Vec<String>,
        #[arg(long)]
        clear_aliases: bool,
        #[arg(long)]
        gender: Option<String>,
        #[arg(long)]
        age: Option<String>,
        #[arg(long)]
        clear_age: bool,
        #[arg(long)]
        role_type: Option<String>,
        #[arg(long)]
        status: Option<String>,
        #[arg(long)]
        description: Option<String>,
        #[arg(long)]
        clear_description: bool,
        #[arg(long = "tag")]
        tags: Vec<String>,
        #[arg(long)]
        clear_tags: bool,
    },
    #[command(about = "删除角色档案")]
    Delete {
        project_id: String,
        character_id: String,
        #[arg(long)]
        confirm: bool,
    },
}

#[derive(Subcommand, Debug)]
#[command(rename_all = "kebab-case")]
pub enum PolyphoneCommands {
    #[command(about = "章节多音字聚合查询")]
    ChapterList {
        project_id: String,
        chapter_id: String,
    },
    #[command(about = "句子多音字列表")]
    SentenceList {
        project_id: String,
        chapter_id: String,
        sentence_id: String,
    },
    #[command(about = "全量保存句子多音字")]
    Save {
        project_id: String,
        chapter_id: String,
        sentence_id: String,
        #[arg(long)]
        annotations_file: PathBuf,
    },
    #[command(about = "清空句子多音字")]
    Clear {
        project_id: String,
        chapter_id: String,
        sentence_id: String,
        #[arg(long)]
        confirm: bool,
    },
    #[command(about = "删除单条多音字")]
    DeleteItem {
        project_id: String,
        chapter_id: String,
        sentence_id: String,
        char_index: i64,
        #[arg(long)]
        confirm: bool,
    },
}

#[derive(Subcommand, Debug)]
#[command(rename_all = "kebab-case")]
pub enum RecycleBinCommands {
    #[command(about = "回收站列表")]
    List {
        #[arg(long, default_value_t = 1)]
        page: usize,
        #[arg(long, default_value_t = 20)]
        page_size: usize,
    },
    #[command(about = "清空回收站")]
    Clear {
        #[arg(long)]
        confirm: bool,
    },
    #[command(about = "恢复项目")]
    Restore { project_id: String },
    #[command(about = "彻底删除项目")]
    Purge {
        project_id: String,
        #[arg(long)]
        confirm: bool,
    },
}

pub async fn execute_cli(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    match cli.command {
        Commands::Project { command } => {
            let (settings, mut store) =
                runtime::build_store(&cli.data_dir, cli.csv_dir.as_deref())?;
            commands::projects::run(&settings, &mut store, command).await?;
        }
        Commands::RecycleBin { command } => {
            let (settings, mut store) =
                runtime::build_store(&cli.data_dir, cli.csv_dir.as_deref())?;
            commands::recycle_bin::run(&settings, &mut store, command).await?;
        }
        Commands::Import { file, title } => {
            let (ctx, manager) =
                crate::setup_cli_runtime(cli.data_dir.to_string_lossy().as_ref()).await?;
            commands::import::run(&ctx, &manager, &file, title.as_deref()).await?;
        }
        Commands::Export {
            project_id,
            format,
            output,
        } => {
            let (ctx, _manager) =
                crate::setup_cli_runtime(cli.data_dir.to_string_lossy().as_ref()).await?;
            commands::export::run(&ctx, &project_id, &format, output.as_deref()).await?;
        }
        Commands::Plugins => {
            let (_ctx, manager) =
                crate::setup_cli_runtime(cli.data_dir.to_string_lossy().as_ref()).await?;
            commands::plugins::run(&manager).await?;
        }
        Commands::Voices => {
            let (ctx, _manager) =
                crate::setup_cli_runtime(cli.data_dir.to_string_lossy().as_ref()).await?;
            commands::voices::run(&ctx).await?;
        }
        Commands::Synthesize {
            project_id,
            chapter,
            voice,
            speed,
            pitch,
        } => {
            let (ctx, _manager) =
                crate::setup_cli_runtime(cli.data_dir.to_string_lossy().as_ref()).await?;
            commands::synthesize::run(&ctx, &project_id, chapter, &voice, speed, pitch).await?;
        }
    }
    Ok(())
}
