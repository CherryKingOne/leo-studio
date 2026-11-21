//! 从音频字节算时长（阶跃星辰返回的是裸音频，不带时长信息）。支持 WAV 与 MP3，其他格式返回 None。

/// 估算音频时长（毫秒）
pub fn probe_duration_ms(bytes: &[u8]) -> Option<u64> {
    if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WAVE" {
        return wav_duration_ms(bytes);
    }
    mp3_duration_ms(bytes)
}

fn wav_duration_ms(bytes: &[u8]) -> Option<u64> {
    let mut pos = 12;
    let mut byte_rate = None;
    while pos + 8 <= bytes.len() {
        let id = &bytes[pos..pos + 4];
        let size = u32::from_le_bytes(bytes[pos + 4..pos + 8].try_into().ok()?) as usize;
        let body = pos + 8;
        if id == b"fmt " && body + 12 <= bytes.len() {
            byte_rate = Some(u32::from_le_bytes(bytes[body + 8..body + 12].try_into().ok()?) as u64);
        }
        if id == b"data" {
            let rate = byte_rate.filter(|r| *r > 0)?;
            // 流式生成的 wav 头里 data 大小可能是占位值，取实际剩余字节数
            let len = size.min(bytes.len().saturating_sub(body)) as u64;
            return Some(len * 1000 / rate);
        }
        pos = body + size + (size & 1);
    }
    None
}

/// 逐帧扫描 MPEG 音频（Layer III），累加每帧采样数
fn mp3_duration_ms(bytes: &[u8]) -> Option<u64> {
    let mut pos = 0;
    // 跳过 ID3v2 标签
    if bytes.len() >= 10 && &bytes[0..3] == b"ID3" {
        let size = ((bytes[6] as usize & 0x7f) << 21)
            | ((bytes[7] as usize & 0x7f) << 14)
            | ((bytes[8] as usize & 0x7f) << 7)
            | (bytes[9] as usize & 0x7f);
        pos = 10 + size;
    }
    let mut total_ms: f64 = 0.0;
    let mut frames = 0u32;
    while pos + 4 <= bytes.len() {
        let h = &bytes[pos..pos + 4];
        if h[0] != 0xFF || h[1] & 0xE0 != 0xE0 {
            pos += 1;
            continue;
        }
        let version = (h[1] >> 3) & 0x03; // 3=MPEG1 2=MPEG2 0=MPEG2.5
        let layer = (h[1] >> 1) & 0x03; // 1=Layer III
        let bitrate_idx = (h[2] >> 4) as usize;
        let sr_idx = ((h[2] >> 2) & 0x03) as usize;
        let padding = ((h[2] >> 1) & 0x01) as usize;
        if version == 1 || layer != 1 || bitrate_idx == 0 || bitrate_idx == 15 || sr_idx == 3 {
            pos += 1;
            continue;
        }
        const BR_V1: [u32; 15] = [0, 32, 40, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320];
        const BR_V2: [u32; 15] = [0, 8, 16, 24, 32, 40, 48, 56, 64, 80, 96, 112, 128, 144, 160];
        const SR_V1: [u32; 3] = [44100, 48000, 32000];
        let (bitrate, sample_rate, samples) = match version {
            3 => (BR_V1[bitrate_idx], SR_V1[sr_idx], 1152u32),
            2 => (BR_V2[bitrate_idx], SR_V1[sr_idx] / 2, 576),
            _ => (BR_V2[bitrate_idx], SR_V1[sr_idx] / 4, 576),
        };
        let frame_len = (samples as usize / 8) * (bitrate as usize * 1000) / sample_rate as usize + padding;
        if frame_len < 4 {
            pos += 1;
            continue;
        }
        total_ms += samples as f64 * 1000.0 / sample_rate as f64;
        frames += 1;
        pos += frame_len;
    }
    (frames > 0).then(|| total_ms.round() as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wav_header() {
        // 24000Hz 16bit 单声道 → 48000 字节/秒；数据 24000 字节 = 500ms
        let mut wav = Vec::new();
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&(36u32 + 24000).to_le_bytes());
        wav.extend_from_slice(b"WAVEfmt ");
        wav.extend_from_slice(&16u32.to_le_bytes());
        wav.extend_from_slice(&1u16.to_le_bytes());
        wav.extend_from_slice(&1u16.to_le_bytes());
        wav.extend_from_slice(&24000u32.to_le_bytes());
        wav.extend_from_slice(&48000u32.to_le_bytes());
        wav.extend_from_slice(&2u16.to_le_bytes());
        wav.extend_from_slice(&16u16.to_le_bytes());
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&24000u32.to_le_bytes());
        wav.extend(std::iter::repeat_n(0u8, 24000));
        assert_eq!(probe_duration_ms(&wav), Some(500));
    }

    #[test]
    fn mp3_frames() {
        // MPEG2 Layer III 24000Hz 48kbps：每帧 576 采样 = 24ms，帧长 144 字节
        let mut mp3 = Vec::new();
        for _ in 0..50 {
            let mut frame = vec![0u8; 144];
            frame[0] = 0xFF;
            frame[1] = 0xF3; // MPEG2, Layer III, no CRC
            frame[2] = 0x64; // bitrate idx 6 (48k), sr idx 1 (24000)
            frame[3] = 0xC4;
            mp3.extend(frame);
        }
        assert_eq!(probe_duration_ms(&mp3), Some(1200));
    }
}
