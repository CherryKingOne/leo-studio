"""项目管理模块的领域模型。

定义项目元数据及状态等领域结构。
"""

from __future__ import annotations

from dataclasses import dataclass
from datetime import datetime
from enum import StrEnum


class ProjectStatus(StrEnum):
    """项目制作状态。"""

    IN_PROGRESS = "制作中"
    READY = "母带就绪"


@dataclass(slots=True)
class ProjectInfo:
    """项目元数据。"""

    project_id: str
    name: str
    file_id: str
    filename: str
    status: str
    chapter_count: int
    total_words: int
    created_at: datetime
    updated_at: datetime


@dataclass(slots=True)
class ChapterOutlineInfo:
    """章节大纲元数据。"""

    chapter_id: str
    chapter_index: int
    chapter_title: str
    word_count: int


@dataclass(slots=True)
class ChapterSentenceInfo:
    """句子切句数据。"""

    sentence_id: str
    sentence_index: int
    content: str
    word_count: int


@dataclass(slots=True)
class ChapterContentInfo:
    """章节详细内容与切句数据。"""

    project_id: str
    file_id: str
    chapter_id: str
    chapter_index: int
    chapter_title: str
    word_count: int
    content: str
    sentences: list[ChapterSentenceInfo]
