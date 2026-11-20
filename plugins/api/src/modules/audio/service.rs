use std::sync::Arc;

use leo_core::ast::AudioFormat;
use leo_core::plugin::{StorageProvider, SynthesisPayload, TTSProvider};

use crate::error::{AppError, AppResult};
use crate::modules::projects::service as project_service;
use crate::state::AppState;
use crate::storage::csv_table::str_field;

use super::schema::{
    AudioClipItem, ChapterSynthesizeRequest, ChapterSynthesizeResponse, PluginItem, VoiceItem,
};

/// 为指定章节批量合成人声音频切片
pub async fn synthesize_chapter(
    state: &AppState,
    project_id: &str,
    chapter_id: &str,
    req: ChapterSynthesizeRequest,
) -> AppResult<ChapterSynthesizeResponse> {
    // 1. 校验项目与章节存在
    {
        let store = state.store.lock().await;
        let _ = project_service::get_project(&store, project_id, false)?;
    }

    // 2. 从 Store 中提取该章节对应的所有切句与对话片段
    let (segments, chapter_title) = {
        let store = state.store.lock().await;
        let chapter_row = store
            .contents
            .find_all()
            .iter()
            .find(|r| str_field(r, "chapter_id") == chapter_id)
            .ok_or_else(|| AppError::file_not_found("章节不存在"))?;
        let chapter_title = str_field(chapter_row, "chapter_title").to_string();

        let chapter_dialogues: Vec<(String, String, String)> = store
            .dialogues
            .find_all()
            .iter()
            .filter(|r| str_field(r, "chapter_id") == chapter_id)
            .map(|r| {
                (
                    str_field(r, "dialogue_id").to_string(),
                    str_field(r, "segment_type").to_string(),
                    str_field(r, "content").to_string(),
                )
            })
            .collect();

        (chapter_dialogues, chapter_title)
    };

    if segments.is_empty() {
        return Err(AppError::invalid_parameter(format!(
            "章节《{chapter_title}》内无可合成的台词或正文片段"
        )));
    }

    // 3. 提取微内核 TTS 与 Storage 服务
    let tts = state
        .context
        .inject::<Arc<dyn TTSProvider>>()
        .await
        .map_err(|e| AppError::internal(format!("未找到 TTS 语音引擎服务: {e}")))?;

    let storage = state
        .context
        .inject::<Arc<dyn StorageProvider>>()
        .await
        .map_err(|e| AppError::internal(format!("未找到 Storage 存储服务: {e}")))?;

    let mut newly_synthesized = 0;
    let mut skipped_count = 0;
    let mut total_duration_ms = 0;
    let mut clips = Vec::new();

    // 4. 逐句进行人声渲染
    for (seg_id, seg_kind, text) in &segments {
        if text.trim().is_empty() {
            skipped_count += 1;
            continue;
        }

        let clip_id = format!("clip_{seg_id}");
        let relative_key = format!("audio/clips/{clip_id}.mp3");

        // 如果未开启强制重新合成，且文件已存在，则复用
        if !req.force && storage.load_audio_clip(&relative_key).await.ok().flatten().is_some() {
            skipped_count += 1;
            clips.push(AudioClipItem {
                clip_id: clip_id.clone(),
                segment_id: seg_id.clone(),
                text: text.clone(),
                kind: seg_kind.clone(),
                voice: req.voice.clone(),
                duration_ms: 1500, // 默认占位估算
                audio_url: format!("/api/v2/audio/{clip_id}"),
            });
            continue;
        }

        // 构造合成载荷
        let payload = SynthesisPayload {
            text: text.clone(),
            voice: req.voice.clone(),
            speed: req.speed,
            pitch: req.pitch,
            emotion: None,
        };

        let output = tts.synthesize(&payload).await.map_err(|err| {
            AppError::internal(format!("合成片段「{text}」失败: {err}"))
        })?;

        let duration = output.duration_ms;
        total_duration_ms += duration;

        // 保存切片文件
        storage
            .save_audio_clip(&clip_id, &output.audio_bytes, AudioFormat::Mp3)
            .await
            .map_err(|err| AppError::storage(format!("保存音频切片失败: {err}")))?;

        newly_synthesized += 1;

        clips.push(AudioClipItem {
            clip_id: clip_id.clone(),
            segment_id: seg_id.clone(),
            text: text.clone(),
            kind: seg_kind.clone(),
            voice: req.voice.clone(),
            duration_ms: duration,
            audio_url: format!("/api/v2/audio/{clip_id}"),
        });
    }

    Ok(ChapterSynthesizeResponse {
        chapter_id: chapter_id.to_string(),
        total_segments: segments.len(),
        newly_synthesized,
        skipped_count,
        total_duration_ms,
        clips,
    })
}

/// 获取单个音频切片的二进制流
pub async fn get_audio_clip(state: &AppState, clip_id: &str) -> AppResult<Vec<u8>> {
    let storage = state
        .context
        .inject::<Arc<dyn StorageProvider>>()
        .await
        .map_err(|e| AppError::internal(format!("未找到 Storage 服务: {e}")))?;

    let relative_key = format!("audio/clips/{clip_id}.mp3");
    let bytes = storage
        .load_audio_clip(&relative_key)
        .await
        .map_err(|e| AppError::storage(format!("读取切片失败: {e}")))?
        .ok_or_else(|| AppError::file_not_found("音频切片不存在或尚未合成"))?;

    Ok(bytes)
}

/// 查询系统当前可用声线
pub async fn list_voices(state: &AppState) -> AppResult<Vec<VoiceItem>> {
    let tts = state
        .context
        .inject::<Arc<dyn TTSProvider>>()
        .await
        .map_err(|e| AppError::internal(format!("未找到 TTS 语音引擎服务: {e}")))?;

    let voices = tts
        .list_voices()
        .await
        .map_err(|e| AppError::internal(format!("获取声线列表失败: {e}")))?;

    Ok(voices
        .into_iter()
        .map(|v| VoiceItem {
            id: v.id,
            name: v.name,
            gender: v.gender,
            locale: v.locale,
            description: v.description,
        })
        .collect())
}

/// 查询已挂载的插件列表
pub async fn list_plugins(state: &AppState) -> AppResult<Vec<PluginItem>> {
    let manifests = state.plugin_manager.list_manifests().await;
    Ok(manifests
        .into_iter()
        .map(|m| PluginItem {
            id: m.id,
            name: m.name,
            version: m.version,
            category: format!("{:?}", m.category).to_lowercase(),
            description: m.description,
            capabilities: m.capabilities,
        })
        .collect())
}
