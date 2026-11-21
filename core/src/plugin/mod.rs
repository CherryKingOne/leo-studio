pub mod manager;
pub mod manifest;
pub mod traits;

pub use manager::{PluginManager, PluginState, PluginStatus};
pub use manifest::{PluginCategory, PluginManifest};
pub use traits::Plugin;

// 兼容旧路径：扩展点契约已移到 `leo_core::spi`
pub use crate::spi::*;
