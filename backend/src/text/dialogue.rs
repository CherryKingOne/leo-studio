#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SegmentType {
    Narration,
    Dialogue,
}

impl SegmentType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Narration => "narration",
            Self::Dialogue => "dialogue",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseStatus {
    Ok,
    UnpairedQuote,
    Empty,
}

impl ParseStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::UnpairedQuote => "unpaired_quote",
            Self::Empty => "empty",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DialogueSegment {
    pub content: String,
    pub segment_type: SegmentType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DialogueLine {
    pub segments: Vec<DialogueSegment>,
    pub status: ParseStatus,
}

pub fn split_dialogue_line(content: &str) -> DialogueLine {
    if content.trim().is_empty() {
        return DialogueLine {
            segments: Vec::new(),
            status: ParseStatus::Empty,
        };
    }

    let (pairs, unmatched) = find_quote_pairs(content);

    if pairs.is_empty() {
        return DialogueLine {
            segments: vec![DialogueSegment {
                content: content.to_string(),
                segment_type: SegmentType::Narration,
            }],
            status: if unmatched {
                ParseStatus::UnpairedQuote
            } else {
                ParseStatus::Ok
            },
        };
    }

    let mut segments = Vec::new();
    let mut cursor = 0;
    for (start, end) in pairs {
        push_segment(
            &mut segments,
            &content[cursor..start],
            SegmentType::Narration,
        );
        push_segment(&mut segments, &content[start..end], SegmentType::Dialogue);
        cursor = end;
    }
    push_segment(&mut segments, &content[cursor..], SegmentType::Narration);

    DialogueLine {
        segments,
        status: if unmatched {
            ParseStatus::UnpairedQuote
        } else {
            ParseStatus::Ok
        },
    }
}

fn find_quote_pairs(content: &str) -> (Vec<(usize, usize)>, bool) {
    let mut pairs = Vec::new();
    let mut open = None;
    let mut unmatched = false;

    for (index, ch) in content.char_indices() {
        match ch {
            '“' => {
                if open.is_none() {
                    open = Some(index);
                }
            }
            '”' => {
                if let Some(start) = open.take() {
                    pairs.push((start, index + ch.len_utf8()));
                } else {
                    unmatched = true;
                }
            }
            _ => {}
        }
    }

    if open.is_some() {
        unmatched = true;
    }

    (pairs, unmatched)
}

fn push_segment(segments: &mut Vec<DialogueSegment>, text: &str, segment_type: SegmentType) {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return;
    }
    segments.push(DialogueSegment {
        content: trimmed.to_string(),
        segment_type,
    });
}
