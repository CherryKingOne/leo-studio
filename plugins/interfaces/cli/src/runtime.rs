use std::path::{Path, PathBuf};

use leo_plugin_api::Settings;
use leo_plugin_api::storage::store::Store;

pub fn build_store(
    data_dir: &Path,
    csv_dir: Option<&Path>,
) -> Result<(Settings, Store), Box<dyn std::error::Error>> {
    let mut settings = Settings::from_env();
    let data_dir = data_dir.to_path_buf();
    settings.data_dir = data_dir.clone();
    settings.upload_dir = data_dir.join("storage/uploads/text-files");
    settings.csv_dir = csv_dir
        .map(Path::to_path_buf)
        .unwrap_or_else(|| data_dir.join(".leo-studio"));
    let store = Store::new(&settings)?;
    Ok((settings, store))
}

pub fn default_csv_dir(data_dir: &Path) -> PathBuf {
    data_dir.join(".leo-studio")
}
