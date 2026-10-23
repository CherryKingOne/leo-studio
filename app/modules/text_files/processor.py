"""小说章节数据处理器：负责章节内容的行级切句、统计与集中化持久化管理。

提供将章节正文按行切分为句子、提取字数、构建切句记录以及与 CSV 表交互的能力。
所有函数与类定义均不使用下划线前缀，符合项目代码风格规范。
"""

from __future__ import annotations

import uuid
from typing import Any

from app.modules.text_files.csv_store import CsvTable
from app.modules.text_files.parsers import count_words


def split_sentences_by_lines(content: str) -> list[str]:
    """将章节正文按行切分为句子列表。

    处理规则：
    - 按换行符切分为多行；
    - 清除每行首尾空白字符（包含半角空格、全角空格、制表符等）；
    - 过滤空白行，仅保留有实际内容的行。
    """
    if not content or not content.strip():
        return []

    sentences: list[str] = []
    for line in content.splitlines():
        cleaned = line.strip()
        if cleaned:
            sentences.append(cleaned)
    return sentences


def build_sentence_records(
    novel_id: str,
    chapter_id: str,
    sentences: list[str],
) -> list[dict[str, str]]:
    """为句子列表构建切句 CSV 记录。

    字段包含：
    - sentence_id: 32 位 UUID 唯一标识
    - novel_id: 所属小说 ID
    - chapter_id: 所属章节 ID
    - sentence_index: 章节内的行/句索引（从 0 开始）
    - content: 句子文本正文
    - word_count: 句子字数（非空白字符数）
    """
    records: list[dict[str, str]] = []
    for idx, sentence in enumerate(sentences):
        records.append({
            "sentence_id": uuid.uuid4().hex,
            "novel_id": novel_id,
            "chapter_id": chapter_id,
            "sentence_index": str(idx),
            "content": sentence,
            "word_count": str(count_words(sentence)),
        })
    return records


class NovelDataProcessor:
    """小说数据集中化处理器。

    负责协调章节内容的文本清洗、切句处理以及切句记录在 CSV 表格中的持久化生命周期。
    """

    def __init__(
        self,
        sentences_table: CsvTable | None = None,
        contents_table: CsvTable | None = None,
    ) -> None:
        self.sentences_table = sentences_table
        self.contents_table = contents_table

    def split_sentences(self, content: str) -> list[str]:
        """将章节正文按行切分为句子列表。"""
        return split_sentences_by_lines(content)

    def create_sentence_records(
        self,
        novel_id: str,
        chapter_id: str,
        content: str,
    ) -> list[dict[str, str]]:
        """对单个章节的内容进行切句并构建对应的记录字典列表。"""
        sentences = self.split_sentences(content)
        return build_sentence_records(
            novel_id=novel_id,
            chapter_id=chapter_id,
            sentences=sentences,
        )

    def process_and_save_chapter(
        self,
        novel_id: str,
        chapter_id: str,
        content: str,
    ) -> list[dict[str, str]]:
        """切分单个章节并将其切句记录落盘至切句 CSV 表格。"""
        if self.sentences_table is None:
            raise ValueError("sentences_table 未配置，无法持久化切句记录")

        records = self.create_sentence_records(
            novel_id=novel_id,
            chapter_id=chapter_id,
            content=content,
        )
        if not records:
            return []
        return self.sentences_table.insert_batch(records)

    def process_and_save_batch(
        self,
        novel_id: str,
        chapters: list[dict[str, Any]],
    ) -> list[dict[str, str]]:
        """批量处理多个章节并将所有切句批量写入切句 CSV 表格。

        chapters 中的每个元素需包含 chapter_id 和 content。
        """
        if self.sentences_table is None:
            raise ValueError("sentences_table 未配置，无法持久化切句记录")

        all_records: list[dict[str, str]] = []
        for ch in chapters:
            chapter_id = str(ch.get("chapter_id", ""))
            content = str(ch.get("content", ""))
            if chapter_id and content:
                records = self.create_sentence_records(
                    novel_id=novel_id,
                    chapter_id=chapter_id,
                    content=content,
                )
                all_records.extend(records)

        if not all_records:
            return []
        return self.sentences_table.insert_batch(all_records)

    def get_sentences_by_chapter(self, chapter_id: str) -> list[dict[str, str]]:
        """获取指定章节的所有切句记录。"""
        if self.sentences_table is None:
            return []
        return self.sentences_table.find_all(lambda r: r.get("chapter_id") == chapter_id)

    def get_sentences_by_novel(self, novel_id: str) -> list[dict[str, str]]:
        """获取指定小说的所有切句记录。"""
        if self.sentences_table is None:
            return []
        return self.sentences_table.find_all(lambda r: r.get("novel_id") == novel_id)

    def delete_sentences_by_novel(self, novel_id: str) -> bool:
        """根据 novel_id 删除关联的所有切句记录。"""
        if self.sentences_table is None:
            return False
        return self.sentences_table.delete_by("novel_id", novel_id)

    def delete_sentences_by_chapter(self, chapter_id: str) -> bool:
        """根据 chapter_id 删除关联的所有切句记录。"""
        if self.sentences_table is None:
            return False
        return self.sentences_table.delete_by("chapter_id", chapter_id)
