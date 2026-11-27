pub mod chapters;
pub mod characters;
pub mod dialogues;
pub mod export;
pub mod import;
pub mod plugins;
pub mod polyphones;
pub mod projects;
pub mod recycle_bin;
pub mod synthesize;
pub mod voices;

pub type CliResult<T> = Result<T, Box<dyn std::error::Error>>;
