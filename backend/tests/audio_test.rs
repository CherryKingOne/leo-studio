mod common;

use axum::http::{Request, StatusCode};
use common::TestApp;

#[tokio::test]
async fn test_voices_and_plugins_api() {
    let app = TestApp::new(50).await;

    // 1. 查询声线列表
    let (status, body) = app.get("/api/v2/voices").await;
    assert_eq!(status, StatusCode::OK);
    let voices = body["data"].as_array().expect("data 必须是数组");
    assert!(!voices.is_empty());
    assert!(voices.iter().any(|v| v["id"] == "zh-CN-YunxiNeural"));
    assert!(voices.iter().any(|v| v["id"] == "zh-CN-XiaoxiaoNeural"));

    // 2. 查询插件清单
    let (status, body) = app.get("/api/v2/plugins").await;
    assert_eq!(status, StatusCode::OK);
    let plugins = body["data"].as_array().expect("plugins 必须是数组");
    assert_eq!(plugins.len(), 5);
    assert!(plugins.iter().any(|p| p["id"] == "leo-plugin-tts-edge"));
    assert!(plugins.iter().any(|p| p["id"] == "leo-plugin-cleaner-kaomoji"));
}

#[tokio::test]
async fn test_chapter_synthesize_and_audio_stream() {
    let app = TestApp::new(50).await;

    // 1. 上传测试小说创建项目
    let sample_novel = "第一章 试炼开始\n林冲冷笑一声：“就凭你，也想夺我造化？”\n言罢，他一拳轰出。\n";
    let (status, body) = app
        .post_multipart(
            "/api/v1/projects",
            &[("name", "音频合成测试项目")],
            Some(("test.txt", "text/plain", sample_novel.as_bytes())),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let project_id = body["data"]["project_id"].as_str().unwrap();

    // 2. 获取章节 ID
    let (status, body) = app.get(&format!("/api/v1/projects/{project_id}/chapters")).await;
    assert_eq!(status, StatusCode::OK);
    let chapters = body["data"]["items"].as_array().unwrap();
    assert_eq!(chapters.len(), 1);
    let chapter_id = chapters[0]["chapter_id"].as_str().unwrap();

    // 3. 调用真正的人声合成接口
    let payload = serde_json::json!({
        "voice": "zh-CN-YunxiNeural",
        "speed": 1.0,
        "pitch": 0.0,
        "force": false
    });
    let (status, body) = app
        .post_json(
            &format!("/api/v2/projects/{project_id}/chapters/{chapter_id}/synthesize"),
            &payload.to_string(),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    let data = &body["data"];
    assert_eq!(data["chapter_id"], chapter_id);
    let clips = data["clips"].as_array().expect("clips 必须是数组");
    assert!(!clips.is_empty());

    let first_clip = &clips[0];
    let clip_id = first_clip["clip_id"].as_str().unwrap();

    // 4. 获取音频切片的二进制流
    let (status, body_str) = app
        .send_raw(
            Request::builder()
                .method("GET")
                .uri(&format!("/api/v2/audio/{clip_id}"))
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(!body_str.is_empty(), "返回的音频数据不能为空");
}
