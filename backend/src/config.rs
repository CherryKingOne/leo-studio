use std::path::{Path, PathBuf};

/// 应用运行期配置。
///
/// 所有配置项均可通过环境变量覆盖，应用级前缀为 `LEO_`，
/// 文本文件模块前缀为 `LEO_TEXT_FILES_`。
#[derive(Debug, Clone)]
pub struct Settings {
    pub app_name: String,
    pub app_version: String,
    pub debug: bool,
    pub cors_origins: Vec<String>,
    pub max_upload_size_mb: usize,
    pub host: String,
    pub port: u16,

    pub data_dir: PathBuf,
    pub upload_dir: PathBuf,
    pub csv_dir: PathBuf,
    pub allowed_extensions: Vec<String>,
    pub module_max_upload_size_mb: Option<usize>,
    pub novels_csv_filename: String,
    pub novel_contents_csv_filename: String,
    pub novel_sentences_csv_filename: String,
    pub novel_dialogues_csv_filename: String,
    pub novel_polyphones_csv_filename: String,
    pub max_filename_length: usize,
}

impl Default for Settings {
    fn default() -> Self {
        Self::from_env()
    }
}

impl Settings {
    /// 从环境变量加载配置。相对路径基于 `LEO_DATA_DIR` 解析。
    pub fn from_env() -> Self {
        let data_dir = env_path("LEO_DATA_DIR", PathBuf::from("."));

        let upload_dir = env_path(
            "LEO_UPLOAD_DIR",
            data_dir.join("storage/uploads/text-files"),
        );
        let csv_dir = env_path("LEO_CSV_DIR", data_dir.join(".leo-studio"));

        Self {
            app_name: env_string("LEO_APP_NAME", "Leo Studio API"),
            app_version: env_string("LEO_APP_VERSION", "0.1.0"),
            debug: env_bool("LEO_DEBUG", true),
            cors_origins: env_list("LEO_CORS_ORIGINS", vec!["*".to_string()]),
            max_upload_size_mb: env_u64("LEO_MAX_UPLOAD_SIZE_MB", 50) as usize,
            host: env_string("LEO_HOST", "0.0.0.0"),
            port: env_u64("LEO_PORT", 58685) as u16,

            data_dir,
            upload_dir,
            csv_dir,
            allowed_extensions: env_list(
                "LEO_TEXT_FILES_ALLOWED_EXTENSIONS",
                vec![".txt".to_string()],
            ),
            module_max_upload_size_mb: env_u64_opt("LEO_TEXT_FILES_MAX_UPLOAD_SIZE_MB")
                .map(|v| v as usize),
            novels_csv_filename: env_string("LEO_TEXT_FILES_NOVELS_CSV_FILENAME", "novels.csv"),
            novel_contents_csv_filename: env_string(
                "LEO_TEXT_FILES_NOVEL_CONTENTS_CSV_FILENAME",
                "novel_contents.csv",
            ),
            novel_sentences_csv_filename: env_string(
                "LEO_TEXT_FILES_NOVEL_SENTENCES_CSV_FILENAME",
                "novel_sentences.csv",
            ),
            novel_dialogues_csv_filename: env_string(
                "LEO_TEXT_FILES_NOVEL_DIALOGUES_CSV_FILENAME",
                "novel_dialogues.csv",
            ),
            novel_polyphones_csv_filename: env_string(
                "LEO_TEXT_FILES_NOVEL_POLYPHONES_CSV_FILENAME",
                "novel_polyphones.csv",
            ),
            max_filename_length: env_u64("LEO_TEXT_FILES_MAX_FILENAME_LENGTH", 255) as usize,
        }
    }

    /// 本模块允许的最大文件大小（字节）。
    pub fn max_bytes(&self) -> usize {
        let mb = self
            .module_max_upload_size_mb
            .unwrap_or(self.max_upload_size_mb);
        mb.saturating_mul(1024 * 1024)
    }
}

fn env_string(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

fn env_bool(key: &str, default: bool) -> bool {
    match std::env::var(key) {
        Ok(value) => matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        ),
        Err(_) => default,
    }
}

fn env_u64(key: &str, default: u64) -> u64 {
    std::env::var(key)
        .ok()
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or(default)
}

fn env_u64_opt(key: &str) -> Option<u64> {
    std::env::var(key).ok().and_then(|v| v.trim().parse().ok())
}

fn env_list(key: &str, default: Vec<String>) -> Vec<String> {
    match std::env::var(key) {
        Ok(value) => value
            .split(',')
            .map(|item| item.trim().to_string())
            .filter(|item| !item.is_empty())
            .collect(),
        Err(_) => default,
    }
}

fn env_path(key: &str, default: PathBuf) -> PathBuf {
    match std::env::var(key) {
        Ok(value) if !value.trim().is_empty() => resolve_path(Path::new(value.trim())),
        _ => resolve_path(&default),
    }
}

/// 将相对路径解析为基于当前工作目录的绝对路径。
pub fn resolve_path(path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    }
}

/// 加载 `.env`：优先当前目录，其次可执行文件上级目录。
pub fn load_dotenv() {
    if dotenvy::dotenv().is_ok() {
        return;
    }
    if let Ok(exe) = std::env::current_exe()
        && let Some(parent) = exe.parent()
    {
        let _ = dotenvy::from_path(parent.join(".env"));
    }
}
