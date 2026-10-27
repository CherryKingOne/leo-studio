"""项目管理服务：负责项目创建、查询与删除生命周期。

通过复用文本文件模块 (app.modules.text_files) 的存储服务与解析器，
实现上传小说的解析、落盘与项目级台账管理。
"""

from __future__ import annotations

import contextlib
import re
import uuid
from datetime import UTC, datetime
from pathlib import Path
from typing import BinaryIO

from app.modules.projects.constants import (
    MAX_PROJECT_VERSIONS,
    PROJECT_VERSIONS_COLUMNS,
    PROJECT_VERSIONS_CSV_FILENAME,
    PROJECTS_COLUMNS,
    PROJECTS_CSV_FILENAME,
)
from app.modules.projects.models import (
    ChapterConflictInfo,
    ChapterContentInfo,
    ChapterOutlineInfo,
    ChapterSentenceInfo,
    ProjectChapterAppendInfo,
    ProjectInfo,
    ProjectRollbackInfo,
    ProjectStatus,
    ProjectVersionInfo,
)
from app.modules.text_files.csv_store import CsvTable
from app.modules.text_files.dependencies import get_storage_service
from app.modules.text_files.parsers import (
    CN_NUM,
    clean_text,
    count_words,
    parse_chapters,
)
from app.modules.text_files.service import TextFileStorageService
from app.shared.errors import (
    FileNotFoundError,
    FileTooLargeError,
    InvalidParameterError,
    StorageError,
)

CHAPTER_NUM_RE = re.compile(
    r"(?:第\s*([\d" + CN_NUM + r"]+)\s*[章回节]|Chapter\s*(\d+))",
    re.IGNORECASE,
)
VOLUME_RE = re.compile(r"第\s*([\d" + CN_NUM + r"]+)\s*[卷篇部]")


def cn_to_int(num_str: str) -> int:
    """将中文数字或阿拉伯数字转换为整型。"""
    cleaned = num_str.strip()
    if cleaned.isdigit():
        return int(cleaned)
    cn_digits = {
        "零": 0, "一": 1, "二": 2, "两": 2, "三": 3,
        "四": 4, "五": 5, "六": 6, "七": 7, "八": 8, "九": 9,
    }
    units = {"十": 10, "百": 100, "千": 1000, "万": 10000}
    if cleaned.startswith("十"):
        cleaned = "一" + cleaned
    val = 0
    curr = 0
    for ch in cleaned:
        if ch in cn_digits:
            curr = cn_digits[ch]
        elif ch in units:
            u = units[ch]
            if u == 10000:
                val = (val + curr) * 10000
                curr = 0
            else:
                val += (curr if curr != 0 else 1) * u
                curr = 0
    val += curr
    return val


def resolve_chapter_key(title: str) -> str:
    """提取章节归一化匹配指纹键，用于防脏数据覆盖比对。

    匹配规则：
    - 带有卷前缀与章节号：提取并归一化为整型键，如 '第一卷 第一章' -> 'v_1_ch_1'
    - 单纯章节号：提取并归一化为整型键，如 '第一章 醒来' / '第1章 醒来' -> 'ch_1'
    - 无明确编号（前言、后记、全文等）：去除空白后作为指纹键
    """
    clean_title = title.strip()
    m_ch = CHAPTER_NUM_RE.search(clean_title)
    m_vol = VOLUME_RE.search(clean_title)

    vol_part = ""
    if m_vol:
        try:
            vol_num = cn_to_int(m_vol.group(1))
            vol_part = f"v_{vol_num}_"
        except Exception:
            pass

    if m_ch:
        ch_raw = m_ch.group(1) or m_ch.group(2)
        if ch_raw:
            try:
                ch_num = cn_to_int(ch_raw)
                return f"{vol_part}ch_{ch_num}"
            except Exception:
                pass

    return clean_title



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
        self.versions_table = CsvTable(
            self.csv_dir / PROJECT_VERSIONS_CSV_FILENAME,
            columns=PROJECT_VERSIONS_COLUMNS,
        )

    def version_record_to_info(self, record: dict[str, str]) -> ProjectVersionInfo:
        """将 CSV 记录转换为 ProjectVersionInfo 领域模型。"""
        created_at_raw = record.get("created_at")
        created_at = (
            datetime.fromisoformat(created_at_raw)
            if created_at_raw
            else datetime.now(UTC)
        )
        return ProjectVersionInfo(
            version_id=record.get("version_id", ""),
            project_id=record.get("project_id", ""),
            file_id=record.get("file_id", ""),
            version_number=int(record.get("version_number", 1) or 1),
            filename=record.get("filename", ""),
            chapter_count=int(record.get("chapter_count", 0) or 0),
            total_words=int(record.get("total_words", 0) or 0),
            created_at=created_at,
            remark=record.get("remark", ""),
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

        # 初始化版本快照 V1
        self.create_project_version(
            project_id=project_id,
            file_id=file_info.file_id,
            filename=file_info.filename,
            version_number=1,
            chapter_count=chapter_count,
            total_words=total_words,
            remark="初始创建",
        )

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
        """删除项目及其关联的文本文件、章节数据与历史版本快照。"""
        info = self.get_project(project_id)

        # 删除项目记录
        self.projects_table.delete_by("project_id", project_id)

        # 清理版本快照记录与附加物理文件
        version_rows = self.versions_table.find_all(
            lambda r: r.get("project_id") == project_id
        )
        self.versions_table.delete_by("project_id", project_id)
        for v in version_rows:
            f_id = v.get("file_id")
            if f_id and f_id != info.file_id:
                for p in self.storage.upload_dir.glob(f"{f_id}.*"):
                    with contextlib.suppress(OSError):
                        p.unlink()

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

    def append_chapters(
        self,
        project_id: str,
        filename: str,
        content: BinaryIO,
        *,
        content_type: str = "text/plain",
        overwrite_existing: bool = False,
        dry_run: bool = False,
    ) -> ProjectChapterAppendInfo:
        """向已有项目安全追加小说章节。

        支持增量追加（如 101~200）与全量重新导入（如 1~200）。
        核心机制：
        1. 差异感知与冲突检测：比对已有章节内容与字数，输出冲突差异列表；
        2. 预检模式（dry_run=True）：仅比对差异与预览结果，不修改任何数据；
        3. 默认非破坏性追加：跳过已有章节，保留原正文与切句，防止劣质爬虫数据污染；
        4. 版本快照与滑动窗口淘汰：每次真实变更生成独立版本快照，超出阈值自动淘汰。
        """
        project = self.get_project(project_id)

        clean_filename = self.storage.sanitize_filename(filename)
        extension = Path(clean_filename).suffix.lower()
        self.storage.check_extension(extension)

        upload_file_id = uuid.uuid4().hex
        target = self.storage.path_for(upload_file_id, extension)

        # 写入物理磁盘作为独立版本快照（绝不物理覆盖旧文件）
        written = 0
        try:
            with target.open("wb") as out:
                while chunk := content.read(8192):
                    written += len(chunk)
                    if written > self.storage.max_bytes():
                        target.unlink(missing_ok=True)
                        raise FileTooLargeError(
                            f"文件大小超出限制（最大 {self.storage.settings.max_file_size_mb} MB）"
                        )
                    out.write(chunk)
        except OSError as exc:
            target.unlink(missing_ok=True)
            raise StorageError(f"保存文件失败：{exc}") from exc

        if written == 0:
            target.unlink(missing_ok=True)
            raise InvalidParameterError("上传文件内容为空")

        # 读取纯文本内容
        content_text = ""
        try:
            content_text = target.read_text(encoding="utf-8")
        except UnicodeDecodeError:
            try:
                content_text = target.read_text(encoding="gb18030")
            except UnicodeDecodeError:
                content_text = target.read_text(encoding="utf-8", errors="replace")

        cleaned = clean_text(content_text)
        new_parsed_chapters = parse_chapters(cleaned)
        if not new_parsed_chapters:
            new_parsed_chapters = [("全文", cleaned or content_text)]

        # 读取该项目现存所有章节
        existing_chapter_rows = self.storage.get_novel_chapters(project.file_id)
        existing_chapter_rows.sort(
            key=lambda r: int(r.get("chapter_index", 0) or 0)
        )

        existing_key_map: dict[str, dict[str, str]] = {}
        for r in existing_chapter_rows:
            key = resolve_chapter_key(r.get("chapter_title", ""))
            existing_key_map[key] = r

        # 比对内容冲突与差异（防线二：差异感知）
        conflicts: list[ChapterConflictInfo] = []
        for ch_title, ch_content in new_parsed_chapters:
            key = resolve_chapter_key(ch_title)
            if key in existing_key_map:
                old_row = existing_key_map[key]
                old_content = old_row.get("content", "")
                old_words = int(old_row.get("word_count", 0) or 0)
                new_words = count_words(ch_content)
                if old_content != ch_content or old_words != new_words:
                    conflicts.append(
                        ChapterConflictInfo(
                            chapter_id=old_row.get("chapter_id", ""),
                            chapter_index=int(old_row.get("chapter_index", 0) or 0),
                            chapter_title=old_row.get("chapter_title", ""),
                            existing_word_count=old_words,
                            incoming_word_count=new_words,
                            word_diff=new_words - old_words,
                        )
                    )

        # 统计新增与跳过章节
        simulated_added = 0
        simulated_skipped = 0
        for ch_title, _ in new_parsed_chapters:
            key = resolve_chapter_key(ch_title)
            if key in existing_key_map:
                simulated_skipped += 1
            else:
                simulated_added += 1

        existing_versions = self.list_project_versions(project.project_id)
        current_max_ver = (
            max(v.version_number for v in existing_versions)
            if existing_versions
            else 1
        )

        # 若为预检模式（dry_run=True），直接清理临时文件并返回比对结果，不写入任何数据
        if dry_run:
            target.unlink(missing_ok=True)
            projected_chapters = (
                len(existing_chapter_rows) + simulated_added
                if not overwrite_existing
                else (
                    len(existing_chapter_rows)
                    + (len(new_parsed_chapters) - simulated_skipped)
                )
            )
            return ProjectChapterAppendInfo(
                project_id=project.project_id,
                file_id=project.file_id,
                version_id="",
                version_number=current_max_ver,
                added_chapters=simulated_added,
                skipped_chapters=simulated_skipped,
                total_chapters=projected_chapters,
                total_words=project.total_words,
                has_conflicts=len(conflicts) > 0,
                conflicts=conflicts,
                updated_at=project.updated_at,
            )

        added_chapters = 0
        skipped_chapters = 0
        new_chapter_records: list[dict[str, str]] = []
        new_sentence_records: list[dict[str, str]] = []

        current_max_index = (
            max(int(r.get("chapter_index", 0) or 0) for r in existing_chapter_rows)
            if existing_chapter_rows
            else -1
        )

        for ch_title, ch_content in new_parsed_chapters:
            key = resolve_chapter_key(ch_title)
            if key in existing_key_map:
                if not overwrite_existing:
                    # 默认策略：跳过已存在章节，保留原高质量正文，防爬虫脏数据污染
                    skipped_chapters += 1
                    continue
                else:
                    # 覆盖模式：更新已有章节内容与切句
                    old_row = existing_key_map[key]
                    old_ch_id = old_row.get("chapter_id", "")
                    w_count = count_words(ch_content)
                    self.storage.contents_table.update_by(
                        "chapter_id",
                        old_ch_id,
                        {
                            "chapter_title": ch_title,
                            "content": ch_content,
                            "word_count": str(w_count),
                        },
                    )
                    # 清理并重写切句
                    self.storage.sentences_table.delete_by("chapter_id", old_ch_id)
                    if ch_content:
                        overwritten_sentences = (
                            self.storage.processor.create_sentence_records(
                                novel_id=project.file_id,
                                chapter_id=old_ch_id,
                                content=ch_content,
                            )
                        )
                        if overwritten_sentences:
                            self.storage.sentences_table.insert_batch(
                                overwritten_sentences
                            )
                    skipped_chapters += 1
                    continue

            # 新增章节
            added_chapters += 1
            current_max_index += 1
            ch_id = uuid.uuid4().hex
            w_count = count_words(ch_content)
            new_chapter_records.append({
                "chapter_id": ch_id,
                "novel_id": project.file_id,
                "chapter_index": str(current_max_index),
                "chapter_title": ch_title,
                "content": ch_content,
                "word_count": str(w_count),
            })
            if ch_content:
                new_sentence_records.extend(
                    self.storage.processor.create_sentence_records(
                        novel_id=project.file_id,
                        chapter_id=ch_id,
                        content=ch_content,
                    )
                )

        if new_chapter_records:
            self.storage.contents_table.insert_batch(new_chapter_records)
        if new_sentence_records:
            self.storage.sentences_table.insert_batch(new_sentence_records)

        # 重新统计该项目的最新总章节数与总字数
        all_chapters = self.storage.get_novel_chapters(project.file_id)
        total_chapters = len(all_chapters)
        total_words = sum(int(r.get("word_count", 0) or 0) for r in all_chapters)

        now = datetime.now(UTC)
        now_iso = now.isoformat()

        # 更新 novels_table
        self.storage.novels_table.update_by(
            "novel_id",
            project.file_id,
            {
                "chapter_count": str(total_chapters),
                "total_words": str(total_words),
            },
        )

        # 更新 projects_table
        self.projects_table.update_by(
            "project_id",
            project.project_id,
            {
                "filename": clean_filename,
                "chapter_count": str(total_chapters),
                "total_words": str(total_words),
                "updated_at": now_iso,
            },
        )

        # 生成新的历史版本快照记录并执行滑动窗口淘汰
        next_ver_num = current_max_ver + 1
        remark = (
            f"追加章节(+{added_chapters})"
            if added_chapters > 0
            else "更新已有章节"
        )
        new_version = self.create_project_version(
            project_id=project.project_id,
            file_id=upload_file_id,
            filename=clean_filename,
            version_number=next_ver_num,
            chapter_count=total_chapters,
            total_words=total_words,
            remark=remark,
        )
        self.prune_project_versions(project.project_id)

        return ProjectChapterAppendInfo(
            project_id=project.project_id,
            file_id=project.file_id,
            version_id=new_version.version_id,
            version_number=new_version.version_number,
            added_chapters=added_chapters,
            skipped_chapters=skipped_chapters,
            total_chapters=total_chapters,
            total_words=total_words,
            has_conflicts=len(conflicts) > 0,
            conflicts=conflicts,
            updated_at=now,
        )

    def create_project_version(
        self,
        project_id: str,
        file_id: str,
        filename: str,
        version_number: int,
        chapter_count: int,
        total_words: int,
        remark: str,
    ) -> ProjectVersionInfo:
        """创建一条项目版本快照记录。"""
        version_id = uuid.uuid4().hex
        now = datetime.now(UTC)
        record = {
            "version_id": version_id,
            "project_id": project_id,
            "file_id": file_id,
            "version_number": str(version_number),
            "filename": filename,
            "chapter_count": str(chapter_count),
            "total_words": str(total_words),
            "created_at": now.isoformat(),
            "remark": remark,
        }
        self.versions_table.insert(record)
        return self.version_record_to_info(record)

    def list_project_versions(self, project_id: str) -> list[ProjectVersionInfo]:
        """查询指定项目的全部历史版本快照，按版本编号倒序排列。"""
        self.get_project(project_id)
        rows = self.versions_table.find_all(
            lambda r: r.get("project_id") == project_id
        )
        items = [self.version_record_to_info(r) for r in rows]
        items.sort(key=lambda v: v.version_number, reverse=True)
        return items

    def prune_project_versions(self, project_id: str) -> None:
        """根据滑动窗口策略淘汰最旧的版本快照，确保最多保留 MAX_PROJECT_VERSIONS 版。"""
        versions = self.list_project_versions(project_id)
        if len(versions) <= MAX_PROJECT_VERSIONS:
            return

        to_prune = versions[MAX_PROJECT_VERSIONS:]
        for v in to_prune:
            self.versions_table.delete_by("version_id", v.version_id)
            # 若无其它版本或项目使用此 file_id，物理删除
            still_version_used = self.versions_table.find_by("file_id", v.file_id)
            still_project_used = self.projects_table.find_by("file_id", v.file_id)
            if not still_version_used and not still_project_used:
                for p in self.storage.upload_dir.glob(f"{v.file_id}.*"):
                    with contextlib.suppress(OSError):
                        p.unlink()

    def rollback_project_version(
        self, project_id: str, version_id: str
    ) -> ProjectRollbackInfo:
        """一键回退到指定历史版本快照。

        1. 验证目标版本与物理文件存在性；
        2. 清除当前已有章节正文与切句；
        3. 从目标版本的物理快照重新解析并写入章节与切句；
        4. 重新计算总字数、章数，并更新项目主表；
        5. 生成一条新的回滚操作版本快照记录并执行滑动窗口淘汰。
        """
        project = self.get_project(project_id)
        target_version_row = self.versions_table.find_by("version_id", version_id)
        if (
            target_version_row is None
            or target_version_row.get("project_id") != project_id
        ):
            raise FileNotFoundError(f"版本快照不存在：{version_id}")

        target_file_id = target_version_row.get("file_id", "")
        target_paths = list(self.storage.upload_dir.glob(f"{target_file_id}.*"))
        if not target_paths or not target_paths[0].exists():
            raise FileNotFoundError("目标版本的物理快照文件不存在或已淘汰")

        target_path = target_paths[0]
        content_text = ""
        try:
            content_text = target_path.read_text(encoding="utf-8")
        except UnicodeDecodeError:
            try:
                content_text = target_path.read_text(encoding="gb18030")
            except UnicodeDecodeError:
                content_text = target_path.read_text(encoding="utf-8", errors="replace")

        cleaned = clean_text(content_text)
        restored_chapters = parse_chapters(cleaned)
        if not restored_chapters:
            restored_chapters = [("全文", cleaned or content_text)]

        # 清除现有章节与切句
        self.storage.contents_table.delete_by("novel_id", project.file_id)
        self.storage.sentences_table.delete_by("novel_id", project.file_id)

        # 写入恢复后的章节与切句
        chapter_records = []
        sentence_records = []
        for idx, (ch_title, ch_content) in enumerate(restored_chapters):
            ch_id = uuid.uuid4().hex
            w_count = count_words(ch_content)
            chapter_records.append({
                "chapter_id": ch_id,
                "novel_id": project.file_id,
                "chapter_index": str(idx),
                "chapter_title": ch_title,
                "content": ch_content,
                "word_count": str(w_count),
            })
            if ch_content:
                sentence_records.extend(
                    self.storage.processor.create_sentence_records(
                        novel_id=project.file_id,
                        chapter_id=ch_id,
                        content=ch_content,
                    )
                )

        if chapter_records:
            self.storage.contents_table.insert_batch(chapter_records)
        if sentence_records:
            self.storage.sentences_table.insert_batch(sentence_records)

        new_total_chapters = len(chapter_records)
        new_total_words = sum(int(r["word_count"]) for r in chapter_records)
        now = datetime.now(UTC)
        now_iso = now.isoformat()

        # 更新 novels_table
        self.storage.novels_table.update_by(
            "novel_id",
            project.file_id,
            {
                "chapter_count": str(new_total_chapters),
                "total_words": str(new_total_words),
            },
        )

        # 更新 projects_table
        restored_filename = target_version_row.get("filename", project.filename)
        self.projects_table.update_by(
            "project_id",
            project.project_id,
            {
                "filename": restored_filename,
                "chapter_count": str(new_total_chapters),
                "total_words": str(new_total_words),
                "updated_at": now_iso,
            },
        )

        # 记录回滚产生的新版本
        existing_versions = self.list_project_versions(project_id)
        next_ver_num = (
            max(v.version_number for v in existing_versions) + 1
            if existing_versions
            else 1
        )
        target_num = int(target_version_row.get("version_number", 1) or 1)
        self.create_project_version(
            project_id=project.project_id,
            file_id=target_file_id,
            filename=restored_filename,
            version_number=next_ver_num,
            chapter_count=new_total_chapters,
            total_words=new_total_words,
            remark=f"回退至版本 V{target_num}",
        )
        self.prune_project_versions(project.project_id)

        return ProjectRollbackInfo(
            project_id=project.project_id,
            restored_version_id=version_id,
            restored_version_number=target_num,
            chapter_count=new_total_chapters,
            total_words=new_total_words,
            updated_at=now,
        )


