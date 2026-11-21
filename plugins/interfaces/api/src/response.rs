use serde::{Deserialize, Serialize};

use crate::error::ErrorCode;

/// 统一响应包装。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub code: i64,
    pub message: String,
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            code: ErrorCode::SUCCESS,
            message: "success".to_string(),
            data: Some(data),
            request_id: None,
        }
    }

    pub fn success_with_message(data: T, message: impl Into<String>) -> Self {
        Self {
            code: ErrorCode::SUCCESS,
            message: message.into(),
            data: Some(data),
            request_id: None,
        }
    }

    pub fn empty() -> Self {
        Self {
            code: ErrorCode::SUCCESS,
            message: "success".to_string(),
            data: None,
            request_id: None,
        }
    }
}
