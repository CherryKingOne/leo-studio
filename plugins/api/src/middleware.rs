use axum::body::{Body, to_bytes};
use axum::extract::Request;
use axum::http::HeaderValue;
use axum::http::header::CONTENT_TYPE;
use axum::middleware::Next;

const REQUEST_ID_HEADER: &str = "X-Request-ID";
const MAX_BODY_BYTES: usize = 128 * 1024 * 1024;

/// 为每个请求生成 request_id，并同时写入响应头与 JSON 响应体。
pub async fn request_id_middleware(request: Request, next: Next) -> axum::response::Response {
    let incoming = request
        .headers()
        .get(REQUEST_ID_HEADER)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.to_string());
    let request_id = incoming.unwrap_or_else(|| uuid::Uuid::new_v4().simple().to_string());

    let response = next.run(request).await;
    let (mut parts, body) = response.into_parts();

    let is_json = parts
        .headers
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.starts_with("application/json"))
        .unwrap_or(false);

    let body = if is_json {
        match to_bytes(body, MAX_BODY_BYTES).await {
            Ok(bytes) => match serde_json::from_slice::<serde_json::Value>(&bytes) {
                Ok(mut value) => {
                    if let Some(object) = value.as_object_mut() {
                        object.insert(
                            "request_id".to_string(),
                            serde_json::Value::String(request_id.clone()),
                        );
                    }
                    Body::from(serde_json::to_vec(&value).unwrap_or(bytes.to_vec()))
                }
                Err(_) => Body::from(bytes),
            },
            Err(_) => Body::empty(),
        }
    } else {
        body
    };

    if let Ok(value) = HeaderValue::from_str(&request_id) {
        parts.headers.insert(REQUEST_ID_HEADER, value);
    }
    axum::response::Response::from_parts(parts, body)
}
