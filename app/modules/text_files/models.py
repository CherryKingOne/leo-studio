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
    DATA = "data"
    CONFIG = "config"
    SOURCE_CODE = "source_code"
    LOG = "log"
    OTHER = "other"


#: 扩展名到归类的映射表
EXTENSION_CATEGORY_MAP: dict[str, TextFileCategory] = {
    ".txt": TextFileCategory.PLAIN_TEXT,
    ".log": TextFileCategory.LOG,
    ".md": TextFileCategory.MARKDOWN,
    ".markdown": TextFileCategory.MARKDOWN,
    ".rst": TextFileCategory.MARKDOWN,
    ".csv": TextFileCategory.DATA,
    ".tsv": TextFileCategory.DATA,
    ".json": TextFileCategory.DATA,
    ".yaml": TextFileCategory.DATA,
    ".yml": TextFileCategory.DATA,
    ".xml": TextFileCategory.DATA,
    ".toml": TextFileCategory.CONFIG,
    ".ini": TextFileCategory.CONFIG,
    ".cfg": TextFileCategory.CONFIG,
    ".conf": TextFileCategory.CONFIG,
    ".env": TextFileCategory.CONFIG,
    ".py": TextFileCategory.SOURCE_CODE,
    ".js": TextFileCategory.SOURCE_CODE,
    ".jsx": TextFileCategory.SOURCE_CODE,
    ".ts": TextFileCategory.SOURCE_CODE,
    ".tsx": TextFileCategory.SOURCE_CODE,
    ".css": TextFileCategory.SOURCE_CODE,
    ".html": TextFileCategory.SOURCE_CODE,
    ".htm": TextFileCategory.SOURCE_CODE,
    ".sql": TextFileCategory.SOURCE_CODE,
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
