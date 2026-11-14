"""小说台词切分：将含中文引号对白的句子拆成叙述段与对白段。

切分只消费切句结果，不修改 ``novel_sentences.csv``；产出的片段记录
写入独立的 ``novel_dialogues.csv``。
"""

from __future__ import annotations

import uuid
from datetime import UTC, datetime

from app.modules.text_files.models import DialogueSegment, DialogueSegmentType
from app.modules.text_files.parsers import count_words

#: 一期支持的中文引号对：开引号 -> 闭引号
OPEN_TO_CLOSE: dict[str, str] = {"“": "”"}


def split_dialogue_line(text: str) -> list[DialogueSegment]:
    """将一行正文按首个完整的中文引号对拆成叙述段与对白段。

    只有同时满足以下条件才拆分：
    - 行内存在成对的 ``“ ”``；
    - 开引号左侧有非空叙述内容。

    整行纯对白、无引号、引号未闭合、或开引号左侧为空时均返回空列表，
    表示该行不产生台词切分记录。
    """
    stripped = text.strip()
    if not stripped:
        return []

    for index, char in enumerate(stripped):
        close_char = OPEN_TO_CLOSE.get(char)
        if close_char is None:
            continue
        if stripped.find(close_char, index + 1) == -1:
            continue

        prefix_raw = stripped[:index]
        prefix = prefix_raw.strip()
        if not prefix:
            return []

        prefix_start = len(prefix_raw) - len(prefix_raw.lstrip())
        return [
            DialogueSegment(
                segment_index=0,
                segment_type=DialogueSegmentType.NARRATION,
                content=prefix,
                char_start=prefix_start,
                char_end=prefix_start + len(prefix),
                quote_char="",
            ),
            DialogueSegment(
                segment_index=1,
                segment_type=DialogueSegmentType.DIALOGUE,
                content=stripped[index:],
                char_start=index,
                char_end=len(stripped),
                quote_char=char,
            ),
        ]

    return []


def build_dialogue_records(
    novel_id: str,
    chapter_id: str,
    sentence_records: list[dict[str, str]],
) -> list[dict[str, str]]:
    """根据切句记录生成台词片段 CSV 记录。"""
    now_iso = datetime.now(UTC).isoformat()
    records: list[dict[str, str]] = []

    for sentence in sentence_records:
        sentence_id = sentence.get("sentence_id", "")
        if not sentence_id:
            continue

        for segment in split_dialogue_line(sentence.get("content", "")):
            records.append({
                "dialogue_id": uuid.uuid4().hex,
                "novel_id": novel_id,
                "chapter_id": chapter_id,
                "sentence_id": sentence_id,
                "sentence_index": sentence.get("sentence_index", "0"),
                "segment_index": str(segment.segment_index),
                "segment_type": str(segment.segment_type),
                "content": segment.content,
                "word_count": str(count_words(segment.content)),
                "char_start": str(segment.char_start),
                "char_end": str(segment.char_end),
                "quote_char": segment.quote_char,
                "created_at": now_iso,
            })

    return records