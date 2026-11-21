use std::sync::LazyLock;

use fancy_regex::{Captures, Regex};

// 颜文字清洗：结构识别 + 强信号校验。
// 与旧版 Python 解析器保持相同的判定规则，避免误删正常正文。

pub const MARKER: char = '\u{e000}';
pub const BLANK_CHARS: &str = " \t\r\n\u{3000}\u{200b}\u{feff}\u{200e}\u{200f}\x0c\x0b";

pub const CN_NUM: &str = "零一二三四五六七八九十百千万两";

const KAOMOJI_RANGES: &[(u32, u32)] = &[
    (0x00A1, 0x00BF),
    (0x00C0, 0x024F),
    (0x0250, 0x02AF),
    (0x02B0, 0x02FF),
    (0x0300, 0x036F),
    (0x0370, 0x03FF),
    (0x0400, 0x052F),
    (0x0530, 0x058F),
    (0x0590, 0x05FF),
    (0x0600, 0x06FF),
    (0x0700, 0x074F),
    (0x0750, 0x077F),
    (0x0900, 0x097F),
    (0x0980, 0x09FF),
    (0x0A00, 0x0A7F),
    (0x0A80, 0x0AFF),
    (0x0B00, 0x0B7F),
    (0x0B80, 0x0BFF),
    (0x0C00, 0x0C7F),
    (0x0C80, 0x0CFF),
    (0x0D00, 0x0D7F),
    (0x0D80, 0x0DFF),
    (0x0E00, 0x0E7F),
    (0x0E80, 0x0EFF),
    (0x0F00, 0x0FFF),
    (0x1000, 0x109F),
    (0x10A0, 0x10FF),
    (0x1100, 0x11FF),
    (0x1200, 0x137F),
    (0x13A0, 0x13FF),
    (0x1400, 0x167F),
    (0x1680, 0x169F),
    (0x16A0, 0x16FF),
    (0x1700, 0x17FF),
    (0x1800, 0x18AF),
    (0x1B00, 0x1B7F),
    (0x1DC0, 0x1DFF),
    (0x1E00, 0x1EFF),
    (0x1F00, 0x1FFF),
    (0x2000, 0x206F),
    (0x2070, 0x209F),
    (0x20A0, 0x20CF),
    (0x20D0, 0x20FF),
    (0x2100, 0x214F),
    (0x2150, 0x218F),
    (0x2190, 0x21FF),
    (0x2200, 0x22FF),
    (0x2300, 0x23FF),
    (0x2460, 0x24FF),
    (0x2500, 0x257F),
    (0x2580, 0x259F),
    (0x25A0, 0x25FF),
    (0x2600, 0x26FF),
    (0x2700, 0x27BF),
    (0x27C0, 0x27EF),
    (0x27F0, 0x27FF),
    (0x2800, 0x28FF),
    (0x2900, 0x297F),
    (0x2980, 0x29FF),
    (0x2A00, 0x2AFF),
    (0x2B00, 0x2BFF),
    (0x2E00, 0x2E7F),
    (0x2E80, 0x2EFF),
    (0x2F00, 0x2FDF),
    (0x3000, 0x303F),
    (0x3040, 0x309F),
    (0x30A0, 0x30FF),
    (0x3100, 0x312F),
    (0x3130, 0x318F),
    (0x31A0, 0x31BF),
    (0x31C0, 0x31EF),
    (0x31F0, 0x31FF),
    (0xAC00, 0xD7AF),
    (0xFE00, 0xFE0F),
    (0xFE10, 0xFE1F),
    (0xFE20, 0xFE2F),
    (0xFE30, 0xFE4F),
    (0xFE50, 0xFE6F),
    (0xFE70, 0xFEFF),
    (0xFF00, 0xFFEF),
    (0x1F300, 0x1F5FF),
    (0x1F600, 0x1F64F),
    (0x1F900, 0x1F9FF),
    (0x1FA70, 0x1FAFF),
];

const FACE_HAN_CHARS: &str = concat!(
    "乂丿丶丷乛亅一二十人八冖几匕匚卩厂厶又口囗土士夕大女子宀寸小尢尸屮山川",
    "工己巾干广廴廾弋弓彐彡彳心戈户手支攵文斗斤方无日曰月木欠止歹殳毋比毛氏",
    "气水火爪父爻爿片牙牛犬玄玉瓜瓦甘生用田疋疒癶白皮皿目矛矢石示禸禾穴立竹",
    "米糸缶网羊羽而耒耳聿肉臣自至臼舌舛舟艮色虍虫血行衣襾角言谷豆豕豸貝贝赤",
    "走足身車车辛辰辵邑酉釆里金長长門门阜隶隹雨青非面革韋韦韭音頁页風风飛飞",
    "食首香馬马骨高髟鬥斗鬯鬲鬼魚鱼鳥鸟鹵卤鹿麥麦麻黃黄黍黑黹黽黾鼎鼓鼠鼻齊",
    "齐齒齿龍龙龜龟龠罒艹灬益曲凸凹〆々〇"
);

const BARE_FACE_HAN_CHARS: &str = "乛皿罒艹灬益曲凸凹〆々〇";

const STRONG_LITERALS: &str = concat!(
    "∀∂∇≧≦≖￢℥℮℧℩Åℬℭℯℰℱℳℴℹ℺℻ℼℽℾℿ⅀⅁⅂⅃⅄ⅅⅆⅇⅈⅉ",
    "ωΩΘΦσΣεΕλΛμΜπΠ",
    "дДзЗэЭ눈",
    "ʘʖʕʔʅʃʬʭʮʯᴥᴗᴖᵔᵕಠಡಥஇ",
    "⌒⌓⌇⌁⌐¬^_•⇀↼‸⚗⚈ˇˍ＠＞＜‿",
    "╮╭╯╰╬╪╫╤╧╩╦╠╣║╔╗╚╝╒╓╕╖╙╛╜╞╟╡╢╣╥╨╪╫╬",
    "︵︶︷︸︹︺︻︼︽︾︿﹀﹁﹂﹃﹄﹉﹊﹋﹌﹍﹎﹏",
    "＿￣＾｀´ˋˊ˙˚˘¯‾⁀⁔",
    "・･゜ﾟ°",
    "●○◎◕◔◉◠◡⊙⊕⊗⊘⊚⊛⊝⊞⊟⊠⊡",
    "☼☀☁☂☃☄☇☈☉☊☋☌☍☎☏",
    "⚗⚘⚙⚚⚛⚜",
    "๑٩۶୧୨๏₀₁₂₃₄₅₆₇₈₉⁰¹²³⁴⁵⁶⁷⁸⁹￩￫",
    "ㅂㅅㅈㅊㅋㅌㅍㅎㅠㅜㅡㅇ",
    "ღლ",
    "乛",
    "〆"
);

const SECONDARY_LITERALS: &str = concat!(
    "=<>~|/\\,.:;'\"",
    "‵′″‴‶‷",
    "·‧∙⋅",
    "ー〜～",
    "ゝゞヽヾノﾉ彡ミ"
);

const WRAP_LITERALS: &str = concat!(
    "╮╭╯╰╬╪╫╤╧╩╦╠╣║╔╗╚╝╒╓╕╖╙╛╜╞╟╡╢╣╥╨╪╫╬",
    "︵︶︷︸︹︺︻︼︽︾︿﹀﹁﹂﹃﹄﹏",
    "ヽヾゝゞノﾉミ彡っッつ",
    "ʕʔლʅʃงฅᕙᕗ٩۶୧୨ო",
    "oObBdDpPqQvVwWxXzZ",
    "☆★♥♡✧✿❀♪♫✪˖˚°·｡ﾟ゜",
    "و",
    "凸_/~\\～〜︴Σ∑εھзЗ┘└┌┐ｼﾍﾞ゛゜꒰꒱づԅㄟㄏႺჂﻌﾞ▁▂▃▄▅▆▇█▉▊▋▌▍▎▏▐░▒▓"
);

const KAOMOJI_ASCII_LITERALS: &str = "_^=<>~|/\\.,:;'\"*#-+$%&@!?oOxXvVmMnNuUtTzZeEyYwWqQpPbBdD";

const BRACKET_FAMILIES: &[(&str, &str)] = &[
    ("(（", ")）"),
    ("[［【〔〖〘〚", "]］】〕〗〙〛"),
    ("{｛﹛", "}｝﹜"),
    ("<〈《", ">〉》"),
    ("「『", "」』"),
    ("｢", "｣"),
    ("﹙", "﹚"),
    ("﹝", "﹞"),
    ("꒰", "꒱"),
];

const ASCII_KAOMOJI_LITERALS: &[&str] = &[
    "orz", "or2", "orƶ", "OTL", "OTZ", "STO", "TAT", "QAQ", "QWQ", "QwQ", "OwO", "UwU", "OvO",
    "owo", "uwu", "T_T", "T.T", "T^T", "Q_Q", "Q.Q", "*_*", "*^*", "x_x", "X_X", "e_e", "n_n",
    "v_v", ";_;", "o_O", "O_o", "o.O", "O.o", ">_<", ">.<", ">o<", "O_O", "o_o", "O_o", "0_0",
    "@_@", "=_=", "-_-", "-__-", "._.", "^_^", "^-^", "^o^", "U_U", "u_u", "= =", "~_~", ":D",
    ":P", ":p", ":)", ":(", ";)", "ㅠㅠ", "ㅜㅜ", "ㅋㅋ",
];

const STRONG_RANGES: &[(u32, u32)] = &[
    (0x2200, 0x23FF),
    (0x2500, 0x257F),
    (0x25A0, 0x25FF),
    (0x2900, 0x297F),
    (0x2B00, 0x2BFF),
];

const SECONDARY_RANGES: &[(u32, u32)] = &[
    (0x2190, 0x21FF),
    (0x2600, 0x26FF),
    (0x2700, 0x27BF),
    (0x2E00, 0x2E7F),
    (0xAC00, 0xD7AF),
];

const BARE_BOUNDARY_CHARS: &str = "，。！？；：、,.!?;:…—“”‘’「」『』\"'";

const WRAPPER_HAN_CHARS: &str = "人山凸凹才";
const FACE_HAN_SIGNAL_CHARS: &str = "灬皿罒艹益曲凸凹";

fn in_ranges(value: u32, ranges: &[(u32, u32)]) -> bool {
    ranges
        .binary_search_by(|(start, end)| {
            if value < *start {
                std::cmp::Ordering::Greater
            } else if value > *end {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Equal
            }
        })
        .is_ok()
}

fn escape_class_char(ch: char) -> String {
    match ch {
        '\\' | ']' | '^' | '-' => format!("\\{ch}"),
        _ => ch.to_string(),
    }
}

fn escape_regex_literal(text: &str) -> String {
    let mut out = String::new();
    for ch in text.chars() {
        if !ch.is_alphanumeric() && ch != '_' {
            out.push('\\');
        }
        out.push(ch);
    }
    out
}

fn build_regex_class(ranges: &[(u32, u32)], literals: &str) -> String {
    let mut parts: Vec<String> = ranges
        .iter()
        .filter_map(|(start, end)| {
            let start = char::from_u32(*start)?;
            let end = char::from_u32(*end)?;
            Some(format!("{start}-{end}"))
        })
        .collect();
    parts.extend(literals.chars().map(escape_class_char));
    parts.concat()
}

fn bracket_open_map() -> Vec<(char, &'static str)> {
    let mut map = Vec::new();
    for (opens, closes) in BRACKET_FAMILIES {
        for ch in opens.chars() {
            map.push((ch, *closes));
        }
    }
    map
}

fn close_chars_for(open: char) -> Option<&'static str> {
    bracket_open_map()
        .into_iter()
        .find(|(ch, _)| *ch == open)
        .map(|(_, closes)| closes)
}

fn open_chars_for(close: char) -> String {
    for (opens, closes) in BRACKET_FAMILIES {
        if closes.chars().any(|ch| ch == close) {
            return opens.to_string();
        }
    }
    String::new()
}

fn bracket_chars() -> String {
    BRACKET_FAMILIES
        .iter()
        .flat_map(|(opens, closes)| opens.chars().chain(closes.chars()))
        .collect()
}

static BARE_CLASS: LazyLock<String> = LazyLock::new(|| {
    build_regex_class(
        KAOMOJI_RANGES,
        &format!("{BARE_FACE_HAN_CHARS}{KAOMOJI_ASCII_LITERALS}"),
    )
});
static STRONG_CLASS: LazyLock<String> =
    LazyLock::new(|| build_regex_class(STRONG_RANGES, STRONG_LITERALS));
static WRAP_CLASS: LazyLock<String> = LazyLock::new(|| build_regex_class(&[], WRAP_LITERALS));

static HAN_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[\u{3400}-\u{4dbf}\u{4e00}-\u{9fff}\u{f900}-\u{faff}]").unwrap());
static STRONG_RE: LazyLock<Regex> = LazyLock::new(|| {
    let class = STRONG_CLASS.as_str();
    Regex::new(&format!("[{class}]")).unwrap()
});

static BARE_RE: LazyLock<Regex> = LazyLock::new(|| {
    let class = BARE_CLASS.as_str();
    Regex::new(&format!("[{class}]{{2,24}}")).unwrap()
});
static BARE_SPACED_RE: LazyLock<Regex> = LazyLock::new(|| {
    let class = BARE_CLASS.as_str();
    Regex::new(&format!(
        "[{class}]{{1,12}}(?:[ \\t\\u{{3000}}]+[{class}]{{1,12}}){{1,3}}"
    ))
    .unwrap_or_else(|err| panic!("bare spaced regex failed: {err:?}; class={class}"))
});
static SPACED_FACE_HINT_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        "[•●○◎◕◔◉◠◡⊙{}ㅂㅅㅈㅊㅋㅌㅍㅎㅠㅜㅡㅇ]",
        BARE_FACE_HAN_CHARS
    ))
    .unwrap()
});
static SLEEPING_FACE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\([:¦]3\s*\[[▓▒░]+\]").unwrap());
static BRACKET_HINT_RE: LazyLock<Regex> = LazyLock::new(|| {
    let opens: String = bracket_open_map().into_iter().map(|(ch, _)| ch).collect();
    let alternation = opens
        .chars()
        .map(|ch| escape_regex_literal(&ch.to_string()))
        .collect::<Vec<_>>()
        .join("|");
    Regex::new(&format!("(?:{alternation})")).unwrap()
});
static ASCII_RE: LazyLock<Regex> = LazyLock::new(|| {
    let mut literals: Vec<&str> = ASCII_KAOMOJI_LITERALS.to_vec();
    literals.sort_by_key(|item| std::cmp::Reverse(item.len()));
    let alternation = literals
        .iter()
        .map(|item| escape_regex_literal(item))
        .collect::<Vec<_>>()
        .join("|");
    Regex::new(&format!(
        r"(?<![A-Za-z0-9])(?:{alternation})(?![A-Za-z0-9])"
    ))
    .unwrap()
});
static WRAPPER_BEFORE_RE: LazyLock<Regex> = LazyLock::new(|| {
    let class = WRAP_CLASS.as_str();
    let marker = escape_regex_literal(&MARKER.to_string());
    Regex::new(&format!(
        r"(?<![A-Za-z0-9])[{class}]{{1,4}}[ \t　]*(?={marker})"
    ))
    .unwrap()
});
static WRAPPER_AFTER_RE: LazyLock<Regex> = LazyLock::new(|| {
    let class = WRAP_CLASS.as_str();
    let marker = escape_regex_literal(&MARKER.to_string());
    Regex::new(&format!(
        r"(?<={marker})[ \t　]*[{class}]{{1,4}}(?![A-Za-z0-9])"
    ))
    .unwrap()
});
static EMPTY_BRACKET_RE: LazyLock<Regex> = LazyLock::new(|| {
    let pairs: &[(&str, &str)] = &[
        ("(", ")"),
        ("（", "）"),
        ("[", "]"),
        ("［", "］"),
        ("【", "】"),
        ("〔", "〕"),
        ("{", "}"),
        ("｛", "｝"),
        ("<", ">"),
        ("〈", "〉"),
        ("《", "》"),
        ("「", "」"),
        ("『", "』"),
        ("〖", "〗"),
        ("〘", "〙"),
        ("〚", "〛"),
        ("｢", "｣"),
        ("﹙", "﹚"),
        ("﹛", "﹜"),
        ("﹝", "﹞"),
    ];
    let marker = escape_regex_literal(&MARKER.to_string());
    let mut alternation = Vec::new();
    for (open, close) in pairs {
        alternation.push(format!(
            "{}[\\s{marker}]*{}",
            escape_regex_literal(open),
            escape_regex_literal(close)
        ));
    }
    Regex::new(&format!("(?:{})+", alternation.join("|"))).unwrap()
});
static SPACE_COLLAPSE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?<=\S)[ \t\u{3000}]{2,}").unwrap());
static CJK_SPACE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?<=[\u{3400}-\u{4dbf}\u{4e00}-\u{9fff}，。！？；：、])[ \t\u{3000}]+(?=[\u{3400}-\u{4dbf}\u{4e00}-\u{9fff}，。！？；：、])",
    )
    .unwrap()
});
static SPACE_BEFORE_PUNCT_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[ \t\u{3000}]+(?=[，。！？；：、])").unwrap());
static QUOTE_PUNCT_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"([“「『])\s*[，。；：、]").unwrap());
static FACE_HAN_SIGNAL_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        "[{}]",
        FACE_HAN_SIGNAL_CHARS
            .chars()
            .map(escape_class_char)
            .collect::<String>()
    ))
    .unwrap()
});

/// 判断字符是否属于常用汉字区。
pub fn is_han(ch: char) -> bool {
    HAN_RE.is_match(&ch.to_string()).unwrap_or(false)
}

fn is_strong_char(ch: char) -> bool {
    in_ranges(ch as u32, STRONG_RANGES) || STRONG_LITERALS.contains(ch)
}

fn is_secondary_char(ch: char) -> bool {
    in_ranges(ch as u32, SECONDARY_RANGES) || SECONDARY_LITERALS.contains(ch)
}

/// 计算颜文字信号分：强信号 2 分，弱信号 1 分。
pub fn signal_score(text: &str) -> i32 {
    text.chars()
        .map(|ch| {
            if is_strong_char(ch) {
                2
            } else if is_secondary_char(ch) {
                1
            } else {
                0
            }
        })
        .sum()
}

fn strong_count(text: &str) -> i32 {
    text.chars().filter(|ch| is_strong_char(*ch)).count() as i32
}

fn face_strong_count(text: &str) -> i32 {
    strong_count(text)
        + text
            .chars()
            .filter(|ch| FACE_HAN_SIGNAL_CHARS.contains(*ch))
            .count() as i32
}

fn face_signal_score(text: &str) -> i32 {
    signal_score(text)
        + 2 * text
            .chars()
            .filter(|ch| FACE_HAN_SIGNAL_CHARS.contains(*ch))
            .count() as i32
}

fn has_face_punctuation(text: &str) -> bool {
    text.chars()
        .any(|ch| !ch.is_whitespace() && !ch.is_alphabetic())
}

fn has_ascii_word(text: &str) -> bool {
    let mut run = 0;
    for ch in text.chars() {
        if ch.is_ascii_alphabetic() {
            run += 1;
            if run >= 2 {
                return true;
            }
        } else {
            run = 0;
        }
    }
    false
}

fn is_face_inner(inner: &str) -> bool {
    let core = inner.trim_matches(|ch: char| BLANK_CHARS.contains(ch));
    if core.is_empty() || core.chars().count() > 30 {
        return false;
    }

    if core
        .chars()
        .any(|ch| is_han(ch) && !FACE_HAN_CHARS.contains(ch))
    {
        return false;
    }

    let ascii_letters: Vec<char> = core.chars().filter(|ch| ch.is_ascii_alphabetic()).collect();
    let ascii_digits: Vec<char> = core.chars().filter(|ch| ch.is_ascii_digit()).collect();
    if !ascii_letters.is_empty() && !ascii_digits.is_empty() {
        return false;
    }
    if ascii_letters
        .iter()
        .any(|ch| !KAOMOJI_ASCII_LITERALS.contains(*ch))
    {
        return false;
    }
    if has_ascii_word(core) && !ASCII_KAOMOJI_LITERALS.contains(&core) {
        return false;
    }

    if face_strong_count(core) == 0 {
        return false;
    }
    let core_len = core.chars().count();
    if !has_face_punctuation(core) && (core_len > 4 || face_signal_score(core) < 4) {
        return false;
    }
    face_signal_score(core) >= 2
}

fn is_bare_face(core: &str) -> bool {
    let core_len = core.chars().count();
    if !(2..=24).contains(&core_len) {
        return false;
    }
    let brackets = bracket_chars();
    if core
        .chars()
        .any(|ch| brackets.contains(ch) && ch != '<' && ch != '>')
    {
        return false;
    }
    if core
        .chars()
        .any(|ch| is_han(ch) && !BARE_FACE_HAN_CHARS.contains(ch))
    {
        return false;
    }
    if has_ascii_word(core) {
        return false;
    }
    if core.chars().any(|ch| ch.is_ascii_digit()) {
        return false;
    }
    if strong_count(core) == 0 {
        return false;
    }
    if !has_face_punctuation(core) && (core_len > 4 || signal_score(core) < 4) {
        return false;
    }
    signal_score(core) >= 2
}

fn bracket_balance_ok(inner: &str, open_chars: &str, close_chars: &str) -> bool {
    let mut depth = 0;
    for ch in inner.chars() {
        if open_chars.contains(ch) {
            depth += 1;
        } else if close_chars.contains(ch) {
            if depth == 0 {
                return false;
            }
            depth -= 1;
        }
    }
    true
}

fn remove_bracket_faces(text: &str) -> (String, bool) {
    if !BRACKET_HINT_RE.is_match(text).unwrap_or(false) {
        return (text.to_string(), false);
    }

    let chars: Vec<char> = text.chars().collect();
    let mut pieces = String::new();
    let mut changed = false;
    let mut index = 0;
    let length = chars.len();

    while index < length {
        let ch = chars[index];
        if let Some(close_chars) = close_chars_for(ch) {
            let limit = std::cmp::min(length, index + 33);
            let mut candidates = Vec::new();
            let mut cursor = index + 1;
            while cursor < limit && chars[cursor] != '\n' {
                if close_chars.contains(chars[cursor]) {
                    candidates.push(cursor);
                }
                cursor += 1;
            }

            let open_chars = open_chars_for(close_chars.chars().next().unwrap_or(' '));
            let mut matched = false;
            for close_index in candidates.into_iter().rev() {
                let inner: String = chars[index + 1..close_index].iter().collect();
                if !bracket_balance_ok(&inner, &open_chars, close_chars) {
                    continue;
                }
                if is_face_inner(&inner) {
                    pieces.push(MARKER);
                    index = close_index + 1;
                    changed = true;
                    matched = true;
                    break;
                }
            }
            if !matched {
                pieces.push(ch);
                index += 1;
            }
            continue;
        }

        pieces.push(ch);
        index += 1;
    }

    (pieces, changed)
}

fn strip_adjacent_wrappers(text: &str) -> String {
    let mut current = text.to_string();
    loop {
        let previous = current.clone();
        current = WRAPPER_BEFORE_RE.replace_all(&current, "").into_owned();
        current = WRAPPER_AFTER_RE.replace_all(&current, "").into_owned();
        if previous == current {
            return current;
        }
    }
}

fn build_bare_replacement(haystack: &str, caps: &Captures<'_>, spaced: bool) -> String {
    let raw = caps.get(0).map(|m| m.as_str()).unwrap_or_default();
    let raw_chars: Vec<char> = raw.chars().collect();

    let mut left = 0;
    while left < raw_chars.len() && BARE_BOUNDARY_CHARS.contains(raw_chars[left]) {
        left += 1;
    }
    let mut right = raw_chars.len();
    while right > left && BARE_BOUNDARY_CHARS.contains(raw_chars[right - 1]) {
        right -= 1;
    }

    let core_raw: String = raw_chars[left..right].iter().collect();
    let compact: String = core_raw
        .chars()
        .filter(|ch| !matches!(ch, ' ' | '\t' | '　'))
        .collect();
    if spaced && !SPACED_FACE_HINT_RE.is_match(&compact).unwrap_or(false) {
        return raw.to_string();
    }
    if compact.chars().count() > 24 || !is_bare_face(&compact) {
        return raw.to_string();
    }

    let left_bytes: usize = raw
        .char_indices()
        .take(left)
        .last()
        .map(|(index, ch)| index + ch.len_utf8())
        .unwrap_or(0);
    let right_bytes: usize = raw
        .char_indices()
        .take(right)
        .last()
        .map(|(index, ch)| index + ch.len_utf8())
        .unwrap_or(0);

    let match_start = caps.get(0).map(|m| m.start()).unwrap_or(0);
    let match_end = caps.get(0).map(|m| m.end()).unwrap_or(0);
    let start = match_start + left_bytes;
    let end = match_start + right_bytes;

    let prev_char = haystack[..start].chars().next_back();
    let next_char = if end <= haystack.len() {
        haystack[end..].chars().next()
    } else {
        None
    };

    if prev_char
        .map(|ch| ch.is_ascii_alphanumeric())
        .unwrap_or(false)
    {
        return raw.to_string();
    }
    if next_char
        .map(|ch| ch.is_ascii_alphanumeric())
        .unwrap_or(false)
    {
        return raw.to_string();
    }
    let _ = match_end;

    format!("{}{}{}", &raw[..left_bytes], MARKER, &raw[right_bytes..])
}

fn replace_bare(text: &str, regex: &Regex, spaced: bool) -> String {
    regex
        .replace_all(text, |caps: &Captures<'_>| {
            build_bare_replacement(text, caps, spaced)
        })
        .into_owned()
}

/// 删除文本中的颜文字片段，保留正常正文与标点。
pub fn strip_kaomoji(text: &str) -> String {
    if text.is_empty() {
        return text.to_string();
    }
    if !STRONG_RE.is_match(text).unwrap_or(false)
        && !ASCII_RE.is_match(text).unwrap_or(false)
        && !SLEEPING_FACE_RE.is_match(text).unwrap_or(false)
        && !(BRACKET_HINT_RE.is_match(text).unwrap_or(false)
            && FACE_HAN_SIGNAL_RE.is_match(text).unwrap_or(false))
    {
        return text.to_string();
    }

    let mut changed = false;
    let mut current = text.to_string();

    let (bracket_removed, removed) = remove_bracket_faces(&current);
    current = bracket_removed;
    changed = changed || removed;
    if changed {
        current = strip_adjacent_wrappers(&current);
    }

    let before = current.clone();
    current = SLEEPING_FACE_RE
        .replace_all(&current, MARKER.to_string())
        .into_owned();
    changed = changed || before != current;

    let before = current.clone();
    current = replace_bare(&current, &BARE_SPACED_RE, true);
    changed = changed || before != current;

    let before = current.clone();
    current = replace_bare(&current, &BARE_RE, false);
    changed = changed || before != current;
    if changed {
        current = strip_adjacent_wrappers(&current);
    }

    let before = current.clone();
    current = ASCII_RE
        .replace_all(&current, MARKER.to_string())
        .into_owned();
    changed = changed || before != current;

    if !changed {
        return current;
    }

    current = strip_adjacent_wrappers(&current);
    let mut previous = String::new();
    while previous != current {
        previous = current.clone();
        current = EMPTY_BRACKET_RE.replace_all(&current, "").into_owned();
    }
    current = current.replace(MARKER, "");
    current = SPACE_COLLAPSE_RE.replace_all(&current, " ").into_owned();
    current = CJK_SPACE_RE.replace_all(&current, "").into_owned();
    current = SPACE_BEFORE_PUNCT_RE.replace_all(&current, "").into_owned();
    current = QUOTE_PUNCT_RE.replace_all(&current, "$1").into_owned();
    current
}

/// 判断已删除颜文字后的剩余文本是否仍属于无实质内容的占位行。
pub fn is_residual_symbol_line(text: &str) -> bool {
    use unicode_categories::UnicodeCategories;

    let s = text.trim_matches(|ch: char| BLANK_CHARS.contains(ch));
    if s.is_empty() {
        return true;
    }

    if STANDALONE_ELLIPSIS_RE.is_match(s).unwrap_or(false)
        || STANDALONE_SEPARATOR_RE.is_match(s).unwrap_or(false)
    {
        return true;
    }

    if s.chars()
        .all(|ch| ch.is_punctuation() || ch.is_symbol() || ch.is_separator() || ch.is_other())
    {
        return true;
    }

    !(HAN_RE.is_match(s).unwrap_or(false)
        || has_ascii_word(s)
        || s.chars().any(|ch| ch.is_ascii_digit()))
}

static STANDALONE_ELLIPSIS_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"^[ \t\u{3000}]*[“\"'‘「『”’]?[ \t\u{3000}]*(?:[.…．⋯‥·•][ \t\u{3000}]*)+[”\"'’」』“‘]?[ \t\u{3000}]*$"#,
    )
    .unwrap()
});

static STANDALONE_SEPARATOR_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"^[ \t\u{3000}]*[“\"'‘「『”’]?[ \t\u{3000}]*(?:[-—–―_－=＝*＊~～#＃][ \t\u{3000}]*){3,}[”\"'’」』“‘]?[ \t\u{3000}]*$"#,
    )
    .unwrap()
});

/// 判断一行文本是否为单独成行的纯标点符号、特殊符号或颜文字占位行。
pub fn is_standalone_symbol_line(text: &str) -> bool {
    is_residual_symbol_line(&strip_kaomoji(text))
}

/// 清洗单行小说文本，返回空字符串表示该行应被丢弃。
pub fn clean_line_text(line: &str) -> String {
    if line.is_empty() {
        return String::new();
    }
    let original = line.trim_matches(|ch: char| BLANK_CHARS.contains(ch));
    if original.is_empty() {
        return String::new();
    }

    let cleaned = strip_kaomoji(line);
    let cleaned = cleaned.trim_matches(|ch: char| BLANK_CHARS.contains(ch));
    if cleaned.is_empty() {
        return String::new();
    }

    if cleaned != original
        && cleaned.chars().count() <= 2
        && cleaned.chars().all(|ch| WRAPPER_HAN_CHARS.contains(ch))
    {
        return String::new();
    }

    if cleaned != original {
        let has_han = HAN_RE.is_match(cleaned).unwrap_or(false);
        let has_word = has_ascii_word(cleaned);
        let has_long_number = {
            let mut run = 0;
            let mut found = false;
            for ch in cleaned.chars() {
                if ch.is_ascii_digit() {
                    run += 1;
                    if run >= 2 {
                        found = true;
                        break;
                    }
                } else {
                    run = 0;
                }
            }
            found
        };
        if !has_han && !has_word && !has_long_number {
            return String::new();
        }
    }

    if is_residual_symbol_line(cleaned) {
        return String::new();
    }
    cleaned.to_string()
}
