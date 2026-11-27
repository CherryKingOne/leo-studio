use leo_core::context::Context;
use leo_plugin_tts::TtsHub;

use super::CliResult;

pub async fn run(ctx: &Context) -> CliResult<()> {
    let voices = TtsHub::of(ctx).await.list_voices().await?;
    println!("声线总数: {}", voices.len());
    for voice in voices {
        println!(
            "{} | {} | {} | {} | {}",
            voice.id, voice.info.name, voice.info.gender, voice.info.locale, voice.info.description
        );
    }
    Ok(())
}
