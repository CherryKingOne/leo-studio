"""文本文件模块的领域模型。

定义模块内部使用的数据结构与分类规则，不依赖任何 Web 框架，
便于独立测试与复用。
"""

from __future__ import annotations

from dataclasses import dataclass
from datetime import datetime
from enum import StrEnum


class TextFileCategory(StrEnum):
    """文本文件归类。

    按内容用途细分，便于前端按类别筛选与展示。
    """

    PLAIN_TEXT = "plain_text"
    MARKDOWN = "markdown"
    WORD = "word"
    EPUB = "epub"
    OTHER = "other"


#: 扩展名到归类的映射表
EXTENSION_CATEGORY_MAP: dict[str, TextFileCategory] = {
    ".txt": TextFileCategory.PLAIN_TEXT,
    ".md": TextFileCategory.MARKDOWN,
    ".doc": TextFileCategory.WORD,
    ".docx": TextFileCategory.WORD,
    ".epub": TextFileCategory.EPUB,
}


def categorize(extension: str) -> TextFileCategory:
    """根据扩展名判断文本文件归类，未知类型归为 ``OTHER``。"""
    return EXTENSION_CATEGORY_MAP.get(extension.lower(), TextFileCategory.OTHER)


@dataclass(slots=True)
class TextFileInfo:
    """已存储文本文件的元数据。"""

    file_id: str
    filename: str
    stored_name: str
    extension: str
    size: int
    content_type: str
    category: TextFileCategory
    uploaded_at: datetime
    download_url: str
    description: str | None = None
