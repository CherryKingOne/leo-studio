use thiserror::Error;

pub type CoreResult<T> = Result<T, CoreError>;

#[derive(Error, Debug, Clone)]
pub enum CoreError {
    #[error("插件错误 [{plugin_id}]: {message}")]
    Plugin {
        plugin_id: String,
        message: String,
    },

    #[error("服务未找到：{0}")]
    ServiceNotFound(String),

    #[error("服务冲突：{service} 已存在名为 {name} 的提供者")]
    ServiceConflict { service: String, name: String },

    #[error("插件冲突：插件 ID {0} 已注册")]
    PluginConflict(String),

    #[error("插件未找到或未挂载：{0}")]
    PluginNotFound(String),

    #[error("插件 {plugin_id} 缺少依赖：{dependency} 未注册")]
    DependencyMissing {
        plugin_id: String,
        dependency: String,
    },

    #[error("插件 {plugin_id} 的依赖 {dependency} 未能挂载")]
    DependencyFailed {
        plugin_id: String,
        dependency: String,
    },

    #[error("插件依赖存在循环：{0}")]
    DependencyCycle(String),

    #[error("管线调度失败：{0}")]
    Pipeline(String),

    #[error("AST 节点未找到 [{node_type}]: {id}")]
    NodeNotFound {
        node_type: &'static str,
        id: String,
    },

    #[error("存储错误：{0}")]
    Storage(String),

    #[error("无效参数：{0}")]
    InvalidParameter(String),

    #[error("内部错误：{0}")]
    Internal(String),
}
