use leo_core::context::Context;
use leo_core::spi::SynthesisPayload;
use leo_plugin_storage::StorageHub;
use leo_plugin_tts::TtsHub;

use super::CliResult;

pub async fn run(
    ctx: &Context,
    project_id: &str,
    chapter_index: usize,
    voice: &str,
    speed: f32,
    pitch: f32,
) -> CliResult<()> {
    let storage = StorageHub::of(ctx)
        .await
        .default_storage()
        .map_err(|err| format!("未挂载存储插件: {err}"))?;
    let tts = TtsHub::of(ctx).await;
    tts.route(voice)
        .map_err(|err| format!("未挂载 TTS 插件: {err}"))?;
    let doc = storage
        .load_script(project_id)
        .await?
        .ok_or_else(|| format!("未找到项目 ID: {project_id}"))?;
    let chapter = doc
        .chapters
        .get(chapter_index)
        .ok_or_else(|| format!("章节索引越界: {chapter_index}"))?;
    let mut clips = 0;
    let mut duration_ms = 0;
    for paragraph in &chapter.paragraphs {
        for sentence in &paragraph.sentences {
            for segment in &sentence.segments {
                let text = segment.text.trim();
                if text.is_empty() {
                    continue;
                }
                let output = tts
                    .synthesize(&SynthesisPayload {
                        text: text.to_string(),
                        voice: voice.to_string(),
                        speed,
                        pitch,
                        emotion: None,
                    })
                    .await?;
                let clip_id = uuid::Uuid::new_v4().simple().to_string();
                storage
                    .save_audio_clip(&clip_id, &output.audio_bytes, output.format)
                    .await?;
                clips += 1;
                duration_ms += output.duration_ms;
            }
        }
    }
    println!("章节: {}", chapter.title);
    println!("切片数: {clips}");
    println!("总时长: {} ms", duration_ms);
    Ok(())
}
