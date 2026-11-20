use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AudioFormat {
    Wav,
    Mp3,
    Opus,
    Flac,
}

impl AudioFormat {
    pub fn extension(&self) -> &'static str {
        match self {
            Self::Wav => "wav",
            Self::Mp3 => "mp3",
            Self::Opus => "opus",
            Self::Flac => "flac",
        }
    }

    pub fn mime_type(&self) -> &'static str {
        match self {
            Self::Wav => "audio/wav",
            Self::Mp3 => "audio/mpeg",
            Self::Opus => "audio/opus",
            Self::Flac => "audio/flac",
        }
    }
}

/// 音频切片元数据引用。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioClipRef {
    pub clip_id: String,
    pub format: AudioFormat,
    pub sample_rate: u32,
    pub duration_ms: u64,
    /// 音频二进制哈希或持久化相对存储路径
    pub storage_key: String,
    /// 生成该切片使用的 TTS 插件 ID 与声线 ID
    pub tts_engine: String,
    pub voice_id: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// 解析 MP3 字节流（MPEG-1/2/2.5 Layer III，可带 ID3v2 标签），按帧累加采样数计算真实时长。
///
/// 无法识别出任何有效帧时返回 `None`；遇到无法解析的尾部数据会停止累加，返回已解析部分的时长。
pub fn mp3_duration_ms(bytes: &[u8]) -> Option<u64> {
    // MPEG-1 Layer III 码率表（kbps），索引 1..=14
    const BITRATES_V1: [u32; 15] = [0, 32, 40, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320];
    // MPEG-2 / 2.5 Layer III 码率表（kbps）
    const BITRATES_V2: [u32; 15] = [0, 8, 16, 24, 32, 40, 48, 56, 64, 80, 96, 112, 128, 144, 160];

    let mut pos = 0usize;
    // 跳过 ID3v2 标签：10 字节头 + synchsafe 长度（+ 可选 10 字节尾）
    if bytes.len() >= 10 && &bytes[0..3] == b"ID3" {
        let size = bytes[6..10]
            .iter()
            .fold(0usize, |acc, b| (acc << 7) | (*b as usize & 0x7f));
        let footer = if bytes[5] & 0x10 != 0 { 10 } else { 0 };
        pos = 10 + size + footer;
    }

    let mut total_samples_ms_num: u64 = 0; // 以 “采样数 × 1000 / 采样率” 累加，避免浮点误差
    let mut frames = 0u64;
    while pos + 4 <= bytes.len() {
        let header = &bytes[pos..pos + 4];
        if header[0] != 0xFF || header[1] & 0xE0 != 0xE0 {
            if frames == 0 {
                // 尚未找到首帧时逐字节同步
                pos += 1;
                continue;
            }
            break;
        }
        let version_bits = (header[1] >> 3) & 0x03; // 00=2.5, 10=2, 11=1
        let layer_bits = (header[1] >> 1) & 0x03; // 01=Layer III
        let bitrate_index = (header[2] >> 4) as usize;
        let sample_rate_index = ((header[2] >> 2) & 0x03) as usize;
        let padding = ((header[2] >> 1) & 0x01) as u32;
        if version_bits == 0b01 || layer_bits != 0b01 || bitrate_index == 0 || bitrate_index == 15 || sample_rate_index == 3 {
            if frames == 0 {
                pos += 1;
                continue;
            }
            break;
        }
        let base_rate = [44_100u32, 48_000, 32_000][sample_rate_index];
        let (sample_rate, bitrate, samples_per_frame, slot_factor) = match version_bits {
            0b11 => (base_rate, BITRATES_V1[bitrate_index], 1152u32, 144u32),
            0b10 => (base_rate / 2, BITRATES_V2[bitrate_index], 576, 72),
            _ => (base_rate / 4, BITRATES_V2[bitrate_index], 576, 72),
        };
        let frame_len = (slot_factor * bitrate * 1000 / sample_rate + padding) as usize;
        if frame_len < 4 || pos + frame_len > bytes.len() {
            // 截断的最后一帧不计入
            break;
        }
        total_samples_ms_num += samples_per_frame as u64 * 1000 * 1_000_000 / sample_rate as u64;
        frames += 1;
        pos += frame_len;
    }

    (frames > 0).then(|| (total_samples_ms_num + 500_000) / 1_000_000)
}

/// 生成音频切片 ID：`clip_{片段ID}_{参数指纹}`。
///
/// 指纹是 TTS 引擎、声线、语速、音调与文本的稳定 64 位 FNV-1a 哈希（16 位十六进制），
/// 同一片段换声线/语速/音调或文本被修改后都会得到不同的切片 ID，从而不会命中旧缓存。
/// REST/MCP 章节合成与 CLI 合成共用此规则，同一参数生成的切片可以互相复用。
pub fn synthesis_clip_id(
    segment_id: &str,
    engine: &str,
    voice: &str,
    speed: f32,
    pitch: f32,
    text: &str,
) -> String {
    const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;
    let mut hash = FNV_OFFSET;
    let speed_bits = speed.to_bits().to_le_bytes();
    let pitch_bits = pitch.to_bits().to_le_bytes();
    let parts: [&[u8]; 5] = [
        engine.as_bytes(),
        voice.as_bytes(),
        &speed_bits,
        &pitch_bits,
        text.as_bytes(),
    ];
    for part in parts {
        // 先写入长度，避免不同字段拼接产生相同字节序列
        for byte in (part.len() as u64).to_le_bytes().iter().chain(part.iter()) {
            hash ^= *byte as u64;
            hash = hash.wrapping_mul(FNV_PRIME);
        }
    }
    format!("clip_{segment_id}_{hash:016x}")
}

#[cfg(test)]
mod tests {
    use super::mp3_duration_ms;

    /// MPEG-2 Layer III、24kHz、48kbps（Edge-TTS 默认格式）：每帧 144 字节、576 个采样 = 24ms
    fn edge_like_frames(count: usize) -> Vec<u8> {
        let mut frame = vec![0u8; 144];
        frame[..4].copy_from_slice(&[0xFF, 0xF3, 0x64, 0xC4]);
        frame.repeat(count)
    }

    #[test]
    fn computes_duration_from_frames() {
        assert_eq!(mp3_duration_ms(&edge_like_frames(50)), Some(1200));
        assert_eq!(mp3_duration_ms(&edge_like_frames(1)), Some(24));
    }

    #[test]
    fn skips_id3_tag_and_ignores_truncated_tail() {
        let mut bytes = b"ID3\x04\x00\x00\x00\x00\x00\x05hello".to_vec();
        bytes.extend(edge_like_frames(10));
        bytes.extend_from_slice(&[0xFF, 0xF3, 0x64]);
        assert_eq!(mp3_duration_ms(&bytes), Some(240));
    }

    #[test]
    fn rejects_non_mp3() {
        assert_eq!(mp3_duration_ms(b""), None);
        assert_eq!(mp3_duration_ms(b"not an mp3 file at all"), None);
    }
}
