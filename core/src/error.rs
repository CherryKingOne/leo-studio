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
