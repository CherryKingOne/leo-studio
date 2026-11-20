use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::{Value, json};

use crate::response::ApiResponse;

/// 业务错误码常量，与旧版 Python 服务保持一致。
pub struct ErrorCode;

impl ErrorCode {
    pub const SUCCESS: i64 = 0;
    pub const INVALID_PARAMETER: i64 = 40000;
    pub const UNAUTHORIZED: i64 = 40100;
    pub const FILE_TOO_LARGE: i64 = 41300;
    pub const UNSUPPORTED_FILE_TYPE: i64 = 41500;
    pub const FILE_NOT_FOUND: i64 = 40400;
    pub const FILE_CONFLICT: i64 = 40900;
    pub const STORAGE_ERROR: i64 = 50000;
    pub const INTERNAL_ERROR: i64 = 50001;
}

/// 统一业务异常。
#[derive(Debug, Clone)]
pub struct AppError {
    pub code: i64,
    pub message: String,
    pub http_status: StatusCode,
    pub details: Option<Value>,
}

impl AppError {
    pub fn new(
        code: i64,
        message: impl Into<String>,
        http_status: StatusCode,
        details: Option<Value>,
    ) -> Self {
        Self {
            code,
            message: message.into(),
            http_status,
            details,
        }
    }

    pub fn invalid_parameter(message: impl Into<String>) -> Self {
        Self::new(
            ErrorCode::INVALID_PARAMETER,
            message,
            StatusCode::UNPROCESSABLE_ENTITY,
            None,
        )
    }

    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self::new(
            ErrorCode::UNAUTHORIZED,
            message,
            StatusCode::UNAUTHORIZED,
            None,
        )
    }

    pub fn file_too_large(message: impl Into<String>) -> Self {
        Self::new(
            ErrorCode::FILE_TOO_LARGE,
            message,
            StatusCode::PAYLOAD_TOO_LARGE,
            None,
        )
    }

    pub fn unsupported_file_type(message: impl Into<String>) -> Self {
        Self::new(
            ErrorCode::UNSUPPORTED_FILE_TYPE,
            message,
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            None,
        )
    }

    pub fn file_not_found(message: impl Into<String>) -> Self {
        Self::new(
            ErrorCode::FILE_NOT_FOUND,
            message,
            StatusCode::NOT_FOUND,
            None,
        )
    }

    pub fn file_conflict(message: impl Into<String>) -> Self {
        Self::new(
            ErrorCode::FILE_CONFLICT,
            message,
            StatusCode::CONFLICT,
            None,
        )
    }

    pub fn storage(message: impl Into<String>) -> Self {
        Self::new(
            ErrorCode::STORAGE_ERROR,
            message,
            StatusCode::INTERNAL_SERVER_ERROR,
            None,
        )
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(
            ErrorCode::INTERNAL_ERROR,
            message,
            StatusCode::INTERNAL_SERVER_ERROR,
            None,
        )
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for AppError {}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let body = ApiResponse::<Value> {
            code: self.code,
            message: self.message,
            data: self.details,
            request_id: None,
        };
        (self.http_status, Json(body)).into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;

/// 便于在 `Result<_, AppError>` 中转换 CSV / IO 错误。
pub fn storage_io(context: &str, err: std::io::Error) -> AppError {
    AppError::new(
        ErrorCode::STORAGE_ERROR,
        format!("{context}：{err}"),
        StatusCode::INTERNAL_SERVER_ERROR,
        Some(json!({
            "context": context,
            "error": err.to_string(),
        })),
    )
}
