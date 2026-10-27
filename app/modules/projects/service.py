"""项目管理服务：负责项目创建、查询与删除生命周期。

通过复用文本文件模块 (app.modules.text_files) 的存储服务与解析器，
实现上传小说的解析、落盘与项目级台账管理。
"""

from __future__ import annotations

import contextlib
import uuid
from datetime import UTC, datetime
from pathlib import Path
from typing import BinaryIO

from app.modules.projects.constants import PROJECTS_COLUMNS, PROJECTS_CSV_FILENAME
from app.modules.projects.models import (
    ChapterContentInfo,
    ChapterOutlineInfo,
    ChapterSentenceInfo,
    ProjectInfo,
    ProjectStatus,
)
from app.modules.text_files.csv_store import CsvTable
from app.modules.text_files.dependencies import get_storage_service
from app.modules.text_files.service import TextFileStorageService
from app.shared.errors import FileNotFoundError, InvalidParameterError


class ProjectService:
    """项目管理业务服务实现。"""

    def __init__(
        self,
        storage_service: TextFileStorageService | None = None,
        csv_dir: Path | str | None = None,
    ) -> None:
        self.storage = storage_service or get_storage_service()
        if csv_dir is not None:
            self.csv_dir = Path(csv_dir)
        else:
            self.csv_dir = self.storage.csv_dir
        self.csv_dir.mkdir(parents=True, exist_ok=True)

        self.projects_table = CsvTable(
            self.csv_dir / PROJECTS_CSV_FILENAME,
            columns=PROJECTS_COLUMNS,
        )

    def record_to_info(self, record: dict[str, str]) -> ProjectInfo:
        """将 CSV 记录转换为 ProjectInfo 领域模型。"""
        created_at_raw = record.get("created_at")
        updated_at_raw = record.get("updated_at")
        created_at = (
            datetime.fromisoformat(created_at_raw) if created_at_raw else datetime.now(UTC)
        )
        updated_at = (
            datetime.fromisoformat(updated_at_raw) if updated_at_raw else created_at
        )

        return ProjectInfo(
            project_id=record.get("project_id", ""),
            name=record.get("name", ""),
            file_id=record.get("file_id", ""),
            filename=record.get("filename", ""),
            status=record.get("status", ProjectStatus.IN_PROGRESS),
            chapter_count=int(record.get("chapter_count", 0) or 0),
            total_words=int(record.get("total_words", 0) or 0),
            created_at=created_at,
            updated_at=updated_at,
        )

    def create_project(
        self,
        name: str,
        filename: str,
        content: BinaryIO,
        *,
        content_type: str = "text/plain",
    ) -> ProjectInfo:
        """创建项目并上传小说文本文件。

        入参仅包含项目名称与上传文件，直接调用文本文件模块的 save_file
        进行校验、清洗、章节解析与切句持久化。
        """
        clean_name = name.strip() if name else ""
        if not clean_name:
            raise InvalidParameterError("项目名称不能为空")

        if not filename or not filename.strip():
            raise InvalidParameterError("上传文件不能为空")

        # 调用文本文件模块的存储服务完成文件落盘、切章、切句
        file_info = self.storage.save_file(
            filename=filename,
            content=content,
            content_type=content_type,
        )

        # 获取章节数与字数
        novel_row = self.storage.novels_table.find_by("novel_id", file_info.file_id)
        chapter_count = int(novel_row.get("chapter_count", 0) or 0) if novel_row else 1
        total_words = int(novel_row.get("total_words", 0) or 0) if novel_row else file_info.size

        # 同步更新小说主表 title 为项目名称
        self.storage.novels_table.update_by("novel_id", file_info.file_id, {"title": clean_name})

        project_id = uuid.uuid4().hex
        now = datetime.now(UTC)
        now_iso = now.isoformat()

        record = {
            "project_id": project_id,
            "name": clean_name,
            "file_id": file_info.file_id,
            "filename": file_info.filename,
            "status": ProjectStatus.IN_PROGRESS,
            "chapter_count": str(chapter_count),
            "total_words": str(total_words),
            "created_at": now_iso,
            "updated_at": now_iso,
        }

        self.projects_table.insert(record)
        return self.record_to_info(record)

    def get_project(self, project_id: str) -> ProjectInfo:
        """根据 project_id 查询项目详情。"""
        record = self.projects_table.find_by("project_id", project_id)
        if record is None:
            raise FileNotFoundError(f"项目不存在：{project_id}")
        return self.record_to_info(record)

    def list_projects(
        self,
        page: int = 1,
        page_size: int = 20,
    ) -> tuple[list[ProjectInfo], int]:
        """分页获取项目列表，按创建时间倒序。"""
        rows = self.projects_table.find_all()
        items = [self.record_to_info(r) for r in rows]
        items.sort(key=lambda x: x.created_at, reverse=True)

        total = len(items)
        start = (page - 1) * page_size
        return items[start : start + page_size], total

    def delete_project(self, project_id: str) -> ProjectInfo:
        """删除项目及其关联的文本文件与章节数据。"""
        info = self.get_project(project_id)

        # 删除项目记录
        self.projects_table.delete_by("project_id", project_id)

        # 级联删除关联的文本文件及章节/切句数据
        with contextlib.suppress(FileNotFoundError):
            self.storage.delete_file(info.file_id)

        return info

    def get_project_chapters(self, project_id: str) -> list[ChapterOutlineInfo]:
        """获取指定项目的章节大纲列表，按章节序号升序排列。"""
        project = self.get_project(project_id)
        rows = self.storage.get_novel_chapters(project.file_id)

        items: list[ChapterOutlineInfo] = [
            ChapterOutlineInfo(
                chapter_id=r.get("chapter_id", ""),
                chapter_index=int(r.get("chapter_index", 0) or 0),
                chapter_title=r.get("chapter_title", ""),
                word_count=int(r.get("word_count", 0) or 0),
            )
            for r in rows
        ]

        items.sort(key=lambda x: x.chapter_index)
        return items

    def get_project_chapter_content(
        self, project_id: str, chapter_id: str
    ) -> ChapterContentInfo:
        """获取指定项目的单个章节详细内容及切句数据。"""
        project = self.get_project(project_id)
        chapter_row = self.storage.contents_table.find_by("chapter_id", chapter_id)
        if chapter_row is None or chapter_row.get("novel_id") != project.file_id:
            raise FileNotFoundError(f"章节不存在：{chapter_id}")

        sentence_rows = self.storage.get_chapter_sentences(chapter_id)
        sentences = [
            ChapterSentenceInfo(
                sentence_id=s.get("sentence_id", ""),
                sentence_index=int(s.get("sentence_index", 0) or 0),
                content=s.get("content", ""),
                word_count=int(s.get("word_count", 0) or 0),
            )
            for s in sentence_rows
        ]
        sentences.sort(key=lambda s: s.sentence_index)

        return ChapterContentInfo(
            project_id=project.project_id,
            file_id=project.file_id,
            chapter_id=chapter_row.get("chapter_id", ""),
            chapter_index=int(chapter_row.get("chapter_index", 0) or 0),
            chapter_title=chapter_row.get("chapter_title", ""),
            word_count=int(chapter_row.get("word_count", 0) or 0),
            content=chapter_row.get("content", ""),
            sentences=sentences,
        )
