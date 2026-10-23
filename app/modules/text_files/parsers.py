"""小说文件名、章节与文本统计解析器。

提供小说文件名元信息解析、章节切分、文本清洗与字数统计能力。
所有函数与类定义均不使用下划线前缀，符合项目代码风格规范。
"""

from __future__ import annotations

import re
from pathlib import Path

# 文件名中的字段分隔符：中文双破折号（两个 U+2014）
FIELD_SEPARATOR = "\u2014\u2014"

# 中文数字字符集
CN_NUM = r"零一二三四五六七八九十百千万两"

# 章节标题正则：行首可选空白 + 章节标记 + 可选标题文字到行尾
CHAPTER_HEADER = re.compile(
    r"^[ \t\u3000]*"
    r"(?:"
    r"第\s*[\d" + CN_NUM + r"]+\s*[章回节卷篇集]"
    r"|Chapter\s+\d+"
    r"|第\s*[\d" + CN_NUM + r"]+\s*部分"
    r")"
    r"(?:[ \t\u3000:：．.、\-—]+.*)?$",
    re.IGNORECASE | re.MULTILINE,
)

# 过滤单独占一行的省略号
STANDALONE_ELLIPSIS = re.compile(r"^[ \t\u3000]*[.…．⋯‥]{2,}[ \t\u3000]*$")

# 过滤单独占一行的分隔线
STANDALONE_SEPARATOR = re.compile(r"^[ \t\u3000]*[-—–_]{3,}[ \t\u3000]*$")

# 空行正则
EMPTY_LINE = re.compile(r"^[ \t\u3000]*$")


def parse_novel_filename(filename: str) -> tuple[str, str, str]:
    """从文件名解析书名、作者与类型。

    支持格式：
    - 书名.txt -> (书名, "", "")
    - 书名——作者.txt -> (书名, 作者, "")
    - 书名——作者——类型.txt -> (书名, 作者, 类型)
    """
    if not filename:
        return "", "", ""

    stem = Path(filename).stem
    parts = [p.strip() for p in stem.split(FIELD_SEPARATOR)]

    if len(parts) == 1:
        return parts[0], "", ""
    if len(parts) == 2:
        return parts[0], parts[1], ""

    title = parts[0]
    author = parts[1]
    genre = FIELD_SEPARATOR.join(p for p in parts[2:] if p)
    return title, author, genre


def clean_text(text: str) -> str:
    """机械清洗小说文本：过滤单独省略号行、单独分隔线行与连续空行。"""
    cleaned_lines: list[str] = []
    for line in text.splitlines():
        if STANDALONE_ELLIPSIS.match(line):
            continue
        if STANDALONE_SEPARATOR.match(line):
            continue
        if EMPTY_LINE.match(line):
            continue
        cleaned_lines.append(line.rstrip())
    return "\n".join(cleaned_lines)


def count_words(text: str) -> int:
    """统计文本字数（非空白字符数，含标点）。"""
    return sum(1 for ch in text if not ch.isspace())


def parse_chapters(text: str) -> list[tuple[str, str]]:
    """按章节标题正则切分小说正文。

    Returns:
        [(章节名称, 章节内容), ...]
        若未识别到章节标题，则返回 [("全文", text.strip())]。
    """
    if not text or not text.strip():
        return []

    matches = list(CHAPTER_HEADER.finditer(text))
    if not matches:
        return [("全文", text.strip())]

    chapters: list[tuple[str, str]] = []
    if matches[0].start() > 0:
        preamble = text[: matches[0].start()].strip()
        if preamble:
            chapters.append(("前言", preamble))

    for i, m in enumerate(matches):
        title = m.group(0).strip()
        content_start = m.end()
        content_end = matches[i + 1].start() if i + 1 < len(matches) else len(text)
        content = text[content_start:content_end].strip()
        chapters.append((title, content))

    return chapters
