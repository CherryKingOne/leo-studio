use axum::extract::Multipart;

use crate::error::{AppError, AppResult};

/// 从 multipart 中读取所有字段。
pub struct MultipartBody {
    pub fields: Vec<(String, String)>,
    pub file_name: Option<String>,
    pub file_content_type: Option<String>,
    pub file_content: Vec<u8>,
}

impl MultipartBody {
    pub fn field(&self, name: &str) -> Option<&str> {
        self.fields
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }

    pub fn bool_field(&self, name: &str, default: bool) -> bool {
        match self.field(name) {
            Some(value) => matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            ),
            None => default,
        }
    }
}

pub async fn read_multipart(mut multipart: Multipart) -> AppResult<MultipartBody> {
    let mut fields = Vec::new();
    let mut file_name = None;
    let mut file_content_type = None;
    let mut file_content = Vec::new();
    let mut found_file = false;

    while let Some(mut field) = multipart
        .next_field()
        .await
        .map_err(|err| AppError::invalid_parameter(format!("表单解析失败：{err}")))?
    {
        let name = field.name().unwrap_or("").to_string();
        if name == "file" {
            found_file = true;
            file_name = field.file_name().map(|value| value.to_string());
            file_content_type = field.content_type().map(|value| value.to_string());
            while let Ok(Some(chunk)) = field.chunk().await {
                file_content.extend_from_slice(&chunk);
            }
        } else {
            let value = field
                .text()
                .await
                .map_err(|err| AppError::invalid_parameter(format!("表单字段读取失败：{err}")))?;
            fields.push((name, value));
        }
    }

    if !found_file {
        return Err(AppError::invalid_parameter("上传文件不能为空"));
    }

    Ok(MultipartBody {
        fields,
        file_name,
        file_content_type,
        file_content,
    })
}
