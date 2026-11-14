use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use leo_studio_backend::app::build_router;
use leo_studio_backend::config::Settings;
use leo_studio_backend::storage::store::Store;
use serde_json::Value;
use tower::ServiceExt;

pub struct TestApp {
    pub router: Router,
}

impl TestApp {
    pub async fn new(max_upload_size_mb: usize) -> Self {
        let base = tempfile::tempdir().unwrap();
        let upload_dir = base.path().join("uploads");
        let csv_dir = base.path().join("csv");
        // 故意泄漏临时目录，保持路径在测试期间有效。
        let base = Box::leak(Box::new(base));

        let settings = Settings {
            app_name: "Leo Studio API".to_string(),
            app_version: "0.1.0".to_string(),
            debug: true,
            cors_origins: vec!["*".to_string()],
            max_upload_size_mb,
            host: "127.0.0.1".to_string(),
            port: 58685,
            data_dir: base.path().to_path_buf(),
            upload_dir: upload_dir.clone(),
            csv_dir: csv_dir.clone(),
            allowed_extensions: vec![".txt".to_string()],
            module_max_upload_size_mb: None,
            novels_csv_filename: "novels.csv".to_string(),
            novel_contents_csv_filename: "novel_contents.csv".to_string(),
            novel_sentences_csv_filename: "novel_sentences.csv".to_string(),
            novel_dialogues_csv_filename: "novel_dialogues.csv".to_string(),
            novel_polyphones_csv_filename: "novel_polyphones.csv".to_string(),
            max_filename_length: 255,
        };

        let store = Store::with_filenames(
            upload_dir.clone(),
            csv_dir.clone(),
            &settings.novels_csv_filename,
            &settings.novel_contents_csv_filename,
            &settings.novel_sentences_csv_filename,
            &settings.novel_dialogues_csv_filename,
            &settings.novel_polyphones_csv_filename,
        )
        .unwrap();

        Self {
            router: build_router(Arc::new(settings), store),
        }
    }

    pub async fn send(&self, request: Request<Body>) -> (StatusCode, Value) {
        let response = self.router.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
        (status, value)
    }

    #[allow(dead_code)]
    pub async fn send_raw(&self, request: Request<Body>) -> (StatusCode, String) {
        let response = self.router.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        (status, String::from_utf8_lossy(&bytes).to_string())
    }

    pub async fn get(&self, path: &str) -> (StatusCode, Value) {
        self.send(
            Request::builder()
                .method("GET")
                .uri(path)
                .body(Body::empty())
                .unwrap(),
        )
        .await
    }

    pub async fn delete(&self, path: &str) -> (StatusCode, Value) {
        self.send(
            Request::builder()
                .method("DELETE")
                .uri(path)
                .body(Body::empty())
                .unwrap(),
        )
        .await
    }

    pub async fn post_multipart(
        &self,
        path: &str,
        fields: &[(&str, &str)],
        file: Option<(&str, &str, &[u8])>,
    ) -> (StatusCode, Value) {
        let (content_type, body) = multipart_body(fields, file);
        self.send(
            Request::builder()
                .method("POST")
                .uri(path)
                .header("content-type", content_type)
                .body(Body::from(body))
                .unwrap(),
        )
        .await
    }

    pub async fn put_json(&self, path: &str, payload: &str) -> (StatusCode, Value) {
        self.send(
            Request::builder()
                .method("PUT")
                .uri(path)
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
    }

    pub async fn patch_json(&self, path: &str, payload: &str) -> (StatusCode, Value) {
        self.send(
            Request::builder()
                .method("PATCH")
                .uri(path)
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
    }
}

pub fn multipart_body(
    fields: &[(&str, &str)],
    file: Option<(&str, &str, &[u8])>,
) -> (String, Vec<u8>) {
    let boundary = "----leotestboundary";
    let mut body = Vec::new();
    for (name, value) in fields {
        body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
        body.extend_from_slice(
            format!("Content-Disposition: form-data; name=\"{name}\"\r\n\r\n").as_bytes(),
        );
        body.extend_from_slice(value.as_bytes());
        body.extend_from_slice(b"\r\n");
    }
    if let Some((filename, content_type, content)) = file {
        body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
        body.extend_from_slice(
            format!("Content-Disposition: form-data; name=\"file\"; filename=\"{filename}\"\r\n")
                .as_bytes(),
        );
        body.extend_from_slice(format!("Content-Type: {content_type}\r\n\r\n").as_bytes());
        body.extend_from_slice(content);
        body.extend_from_slice(b"\r\n");
    }
    body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
    (format!("multipart/form-data; boundary={boundary}"), body)
}
