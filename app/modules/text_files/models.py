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

    【新增格式指南】：
    如果新增的文件格式需要专属归类标识，在此处添加新的枚举值，例如：
        MARKDOWN = "markdown"
        WORD = "word"
        EPUB = "epub"
    若无需特殊分类，可直接沿用 PLAIN_TEXT 或 OTHER。
    """

    PLAIN_TEXT = "plain_text"
    OTHER = "other"


#: 扩展名到归类的映射表
#: 【新增格式指南】：
#: 在下方映射表中添加新扩展名到分类的映射，例如：
#:     ".md": TextFileCategory.MARKDOWN,
#: 未配置的扩展名在通过 categorize() 时会自动归入 TextFileCategory.OTHER。
EXTENSION_CATEGORY_MAP: dict[str, TextFileCategory] = {
    ".txt": TextFileCategory.PLAIN_TEXT,
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
