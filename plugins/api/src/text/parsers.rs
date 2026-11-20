use std::collections::HashMap;
use std::path::Path;
use std::sync::LazyLock;

use fancy_regex::Regex;

pub use crate::text::kaomoji::{BLANK_CHARS, CN_NUM, clean_line_text, is_han, strip_kaomoji};

const FIELD_SEPARATOR: &str = "\u{2014}\u{2014}";

static CHAPTER_HEADER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        r#"(?im)^[ \t\u3000]*(?:第\s*[\d{CN_NUM}]+\s*[章回节卷篇集]|Chapter\s+\d+|第\s*[\d{CN_NUM}]+\s*部分)(?:[ \t\u3000:：．.、\-—]+.*)?$"#
    ))
    .unwrap()
});

static CHAPTER_NUM_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        r"(?i)(?:第\s*([\d{CN_NUM}]+)\s*[章回节]|Chapter\s*(\d+))"
    ))
    .unwrap()
});

static VOLUME_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(&format!(r"第\s*([\d{CN_NUM}]+)\s*[卷篇部]")).unwrap());

/// 机械清洗小说文本。
pub fn clean_text(text: &str) -> String {
    let mut lines = Vec::new();
    for line in text.lines() {
        let cleaned = clean_line_text(line);
        if !cleaned.is_empty() {
            lines.push(cleaned);
        }
    }
    lines.join("\n")
}

/// 统计文本字数（非空白字符数，含标点）。
pub fn count_words(text: &str) -> usize {
    text.chars().filter(|ch| !ch.is_whitespace()).count()
}

/// 从文件名解析书名、作者与类型。
pub fn parse_novel_filename(filename: &str) -> (String, String, String) {
    if filename.is_empty() {
        return (String::new(), String::new(), String::new());
    }

    let stem = Path::new(filename)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    let parts: Vec<String> = stem
        .split(FIELD_SEPARATOR)
        .map(|part| part.trim().to_string())
        .collect();

    match parts.len() {
        0 | 1 => (
            parts.first().cloned().unwrap_or_default(),
            String::new(),
            String::new(),
        ),
        2 => (parts[0].clone(), parts[1].clone(), String::new()),
        _ => {
            let title = parts[0].clone();
            let author = parts[1].clone();
            let genre = parts[2..]
                .iter()
                .filter(|part| !part.is_empty())
                .cloned()
                .collect::<Vec<_>>()
                .join(FIELD_SEPARATOR);
            (title, author, genre)
        }
    }
}

/// 按章节标题正则切分小说正文。
pub fn parse_chapters(text: &str) -> Vec<(String, String)> {
    if text.trim().is_empty() {
        return Vec::new();
    }

    let matches: Vec<_> = CHAPTER_HEADER
        .find_iter(text)
        .filter_map(|m| m.ok())
        .map(|m| (m.start(), m.end(), m.as_str().to_string()))
        .collect();
    if matches.is_empty() {
        return vec![("全文".to_string(), text.trim().to_string())];
    }

    let mut chapters = Vec::new();
    if matches[0].0 > 0 {
        let preamble = text[..matches[0].0].trim();
        if !preamble.is_empty() {
            chapters.push(("前言".to_string(), preamble.to_string()));
        }
    }

    for (index, (_, end, title)) in matches.iter().enumerate() {
        let content_end = matches
            .get(index + 1)
            .map(|(start, _, _)| *start)
            .unwrap_or(text.len());
        let content = text[*end..content_end].trim();
        chapters.push((title.clone(), content.to_string()));
    }

    chapters
}

/// 将中文数字或阿拉伯数字转换为整型。
pub fn cn_to_int(num_str: &str) -> Option<i64> {
    let cleaned = num_str.trim();
    if cleaned.is_empty() {
        return None;
    }
    if let Ok(value) = cleaned.parse::<i64>() {
        return Some(value);
    }

    let digits: HashMap<char, i64> = [
        ('零', 0),
        ('一', 1),
        ('二', 2),
        ('两', 2),
        ('三', 3),
        ('四', 4),
        ('五', 5),
        ('六', 6),
        ('七', 7),
        ('八', 8),
        ('九', 9),
    ]
    .into_iter()
    .collect();
    let units: HashMap<char, i64> = [('十', 10), ('百', 100), ('千', 1000), ('万', 10000)]
        .into_iter()
        .collect();

    let mut cleaned = cleaned.to_string();
    if cleaned.starts_with('十') {
        cleaned = format!("一{cleaned}");
    }

    let mut value = 0i64;
    let mut current = 0i64;
    for ch in cleaned.chars() {
        if let Some(digit) = digits.get(&ch) {
            current = *digit;
        } else if let Some(unit) = units.get(&ch) {
            if *unit == 10000 {
                value = (value + current) * 10000;
                current = 0;
            } else {
                value += if current != 0 { current } else { 1 } * unit;
                current = 0;
            }
        }
    }
    value += current;
    Some(value)
}

/// 提取章节标题中的章节序号，支持阿拉伯数字与中文数字。
pub fn chapter_number(title: &str) -> Option<i64> {
    let captures = CHAPTER_NUM_RE.captures(title.trim()).ok().flatten()?;
    let raw = captures.get(1).or_else(|| captures.get(2))?;
    cn_to_int(raw.as_str())
}

/// 提取章节归一化匹配指纹键，用于防脏数据覆盖比对。
pub fn resolve_chapter_key(title: &str) -> String {
    let clean_title = title.trim();
    let chapter = CHAPTER_NUM_RE.captures(clean_title).ok().flatten();
    let volume = VOLUME_RE.captures(clean_title).ok().flatten();

    let mut volume_part = String::new();
    if let Some(volume) = volume.as_ref()
        && let Some(raw) = volume.get(1)
        && let Some(number) = cn_to_int(raw.as_str())
    {
        volume_part = format!("v_{number}_");
    }

    if let Some(chapter) = chapter.as_ref() {
        let raw = chapter
            .get(1)
            .or_else(|| chapter.get(2))
            .map(|m| m.as_str());
        if let Some(raw_str) = raw
            && let Some(number) = cn_to_int(raw_str)
        {
            return format!("{volume_part}ch_{number}");
        }
    }

    clean_title.to_string()
}

fn tone_mark(ch: char) -> Option<(&'static str, i32)> {
    Some(match ch {
        'ā' => ("a", 1),
        'á' => ("a", 2),
        'ǎ' => ("a", 3),
        'à' => ("a", 4),
        'ō' => ("o", 1),
        'ó' => ("o", 2),
        'ǒ' => ("o", 3),
        'ò' => ("o", 4),
        'ē' => ("e", 1),
        'é' => ("e", 2),
        'ě' => ("e", 3),
        'è' => ("e", 4),
        'ī' => ("i", 1),
        'í' => ("i", 2),
        'ǐ' => ("i", 3),
        'ì' => ("i", 4),
        'ū' => ("u", 1),
        'ú' => ("u", 2),
        'ǔ' => ("u", 3),
        'ù' => ("u", 4),
        'ǖ' => ("v", 1),
        'ǘ' => ("v", 2),
        'ǚ' => ("v", 3),
        'ǜ' => ("v", 4),
        'ü' => ("v", 5),
        _ => return None,
    })
}

/// 解析拼音的基准字母、声调数字与 TTS 拼音表示。
pub fn parse_pinyin_elements(pinyin: &str) -> (String, i32, String) {
    let cleaned = pinyin.trim().to_lowercase();
    if cleaned.is_empty() {
        return (String::new(), 5, String::new());
    }

    let last = cleaned.chars().last().unwrap();
    if last.is_ascii_digit() && ('0'..='5').contains(&last) {
        let tone = last.to_digit(10).unwrap() as i32;
        let base = &cleaned[..cleaned.len() - last.len_utf8()];
        return (base.to_string(), tone, format!("{base}{tone}"));
    }

    let mut detected_tone = 0;
    let mut base_chars = String::new();
    for ch in cleaned.chars() {
        if let Some((base, tone)) = tone_mark(ch) {
            base_chars.push_str(base);
            detected_tone = tone;
        } else {
            base_chars.push(ch);
        }
    }
    if detected_tone == 0 {
        detected_tone = 5;
    }

    let tts = format!("{base_chars}{detected_tone}");
    (base_chars, detected_tone, tts)
}
