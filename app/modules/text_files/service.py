"""文本文件存储服务：负责落盘、校验、CSV 小说元数据与章节内容管理。

设计要点：

- 在 ``.leo-studio/novels.csv`` 中管理已上传小说的业务元信息：
  包含 novel_id, title, author, genre, chapter_count, total_words, avg_words_per_chapter, line_count, created_at。
  不包含底层文件字段（stored_name、content_type、category、description）。
- 在 ``.leo-studio/novel_contents.csv`` 中按章节拆分存储小说正文内容：
  包含 chapter_id, novel_id, chapter_index, chapter_title, content, word_count，
  通过 novel_id 关联到主表。
"""

from __future__ import annotations

import re
import shutil
import uuid
from collections.abc import Iterable
from datetime import UTC, datetime
from pathlib import Path
from typing import BinaryIO

from app.modules.text_files.config import TextFilesSettings, get_text_files_settings
from app.modules.text_files.constants import (
    BINARY_EXTENSIONS,
    EXTENSION_MIME_MAP,
    NOVEL_CONTENTS_COLUMNS,
    NOVELS_COLUMNS,
)
from app.modules.text_files.csv_store import CsvTable
from app.modules.text_files.models import TextFileInfo, categorize
from app.modules.text_files.parsers import (
    clean_text,
    count_words,
    parse_chapters,
    parse_novel_filename,
)
from app.shared.config import get_settings
from app.shared.errors import (
    FileNotFoundError,
    FileTooLargeError,
    InvalidParameterError,
    StorageError,
    UnsupportedFileTypeError,
)

SAFE_NAME_RE = re.compile(r"[^\w一-鿿\-.—·]")


def human_size(size: int) -> str:
    """将字节数转换为可读字符串。"""
    value = float(size)
    for unit in ("B", "KB", "MB", "GB", "TB"):
        if value < 1024:
            return f"{int(value)} B" if unit == "B" else f"{value:.2f} {unit}"
        value /= 1024
    return f"{value:.2f} TB"


class TextFileStorageService:
    """文本文件的本地文件系统与 CSV 小说章节存储实现。"""

    def __init__(self, settings: TextFilesSettings | None = None) -> None:
        self.settings = settings or get_text_files_settings()
        self.upload_dir = Path(self.settings.upload_dir)
        self.upload_dir.mkdir(parents=True, exist_ok=True)

        self.csv_dir = Path(self.settings.csv_dir)
        self.csv_dir.mkdir(parents=True, exist_ok=True)

        self.novels_table = CsvTable(
            self.csv_dir / self.settings.novels_csv_filename,
            columns=NOVELS_COLUMNS,
        )
        self.contents_table = CsvTable(
            self.csv_dir / self.settings.novel_contents_csv_filename,
            columns=NOVEL_CONTENTS_COLUMNS,
        )

    # ------------------------------------------------------------------
    # 名称与路径处理
    # ------------------------------------------------------------------
    def sanitize_filename(self, filename: str) -> str:
        """清洗上传文件名，移除路径分隔符与危险字符。"""
        if not filename or not filename.strip():
            raise InvalidParameterError("文件名不能为空")
        # 同时兼容 POSIX 与 Windows 路径分隔符。
        name = filename.replace("\\", "/").split("/")[-1]
        name = SAFE_NAME_RE.sub("_", name).strip("._ ")
        if not name:
            raise InvalidParameterError("文件名不合法")
        if len(name) > self.settings.max_filename_length:
            raise InvalidParameterError(
                f"文件名长度不能超过 {self.settings.max_filename_length} 个字符"
            )
        return name

    def check_extension(self, extension: str) -> None:
        """校验扩展名是否在文本文件白名单内。"""
        allowed = {ext.lower() for ext in self.settings.allowed_extensions}
        if allowed and extension.lower() not in allowed:
            raise UnsupportedFileTypeError(
                f"不支持的文件类型 {extension}，允许的类型：{', '.join(sorted(allowed))}"
            )

    @staticmethod
    def resolve_content_type(extension: str, content_type: str | None) -> str:
        """确定文件的 MIME 类型。"""
        mapped = EXTENSION_MIME_MAP.get(extension.lower())
        if mapped is not None:
            return mapped
        if content_type and content_type.strip():
            return content_type.strip()
        return "application/octet-stream"

    @staticmethod
    def is_previewable(extension: str) -> bool:
        """判断该扩展名的文件是否支持文本内容预览。"""
        return extension.lower() not in BINARY_EXTENSIONS

    def path_for(self, file_id: str, extension: str) -> Path:
        return self.upload_dir / f"{file_id}{extension.lower()}"

    def max_bytes(self) -> int:
        """本模块允许的最大文件大小（字节）。"""
        if self.settings.max_upload_size_mb is not None:
            return self.settings.max_upload_size_mb * 1024 * 1024
        return get_settings().max_upload_size_bytes

    # ------------------------------------------------------------------
    # 兼容性索引接口
    # ------------------------------------------------------------------
    def load_index(self) -> dict[str, dict]:
        """从 novels.csv 加载索引字典，保持向后兼容。"""
        rows = self.novels_table.find_all()
        return {r.get("novel_id", ""): r for r in rows}

    def save_index(self, index: dict[str, dict]) -> None:
        """向 novels.csv 批量覆写索引记录，保持向后兼容。"""
        self.novels_table.write_rows(list(index.values()))

    # ------------------------------------------------------------------
    # 核心操作
    # ------------------------------------------------------------------
    def save_file(
        self,
        filename: str,
        content: BinaryIO,
        *,
        content_type: str = "text/plain",
        description: str | None = None,
    ) -> TextFileInfo:
        """保存单个文本文件并解析章节写入 CSV 表。

        写入时按块读取并实时校验大小，超限立即中断并删除半成品文件。
        纯文本类型会自动识别章节标题并逐章写入 novel_contents.csv。
        """
        parsed_title, _, _ = parse_novel_filename(filename)
        safe_name = self.sanitize_filename(filename)
        extension = Path(safe_name).suffix.lower()
        self.check_extension(extension)

        title = parsed_title or Path(safe_name).stem
        file_format = extension.lstrip(".")

        file_id = uuid.uuid4().hex
        target = self.path_for(file_id, extension)
        max_bytes = self.max_bytes()
        written = 0

        try:
            with target.open("wb") as target_file:
                while chunk := content.read(1024 * 1024):
                    written += len(chunk)
                    if written > max_bytes:
                        raise FileTooLargeError(
                            f"文件大小超过限制，最大允许 {human_size(max_bytes)}"
                        )
                    target_file.write(chunk)
        except FileTooLargeError:
            target.unlink(missing_ok=True)
            raise
        except OSError as exc:
            target.unlink(missing_ok=True)
            raise StorageError(f"保存文件失败：{exc}") from exc

        if written == 0:
            target.unlink(missing_ok=True)
            raise InvalidParameterError("上传文件内容为空")

        # 读取纯文本文件内容以便切分章节
        content_text = ""
        chapters: list[tuple[str, str]] = []
        if self.is_previewable(extension):
            try:
                content_text = target.read_text(encoding="utf-8")
            except UnicodeDecodeError:
                try:
                    content_text = target.read_text(encoding="gb18030")
                except UnicodeDecodeError:
                    content_text = target.read_text(encoding="utf-8", errors="replace")

            cleaned = clean_text(content_text)
            chapters = parse_chapters(cleaned)
            if not chapters:
                chapters = [("全文", cleaned or content_text)]
        else:
            chapters = [("二进制文档", "")]

        chapter_count = len(chapters)
        total_words = count_words(content_text) if content_text else written
        line_count = sum(1 for line in content_text.splitlines() if line.strip()) if content_text else 0
        now = datetime.now(UTC)

        info = TextFileInfo(
            file_id=file_id,
            filename=safe_name,
            stored_name=target.name,
            extension=extension,
            size=written,
            content_type=self.resolve_content_type(extension, content_type),
            category=categorize(extension),
            uploaded_at=now,
            download_url=f"/api/v1/text-files/{file_id}/download",
            description=description,
        )

        # 写入小说主表 (novels.csv)
        self.novels_table.insert({
            "novel_id": file_id,
            "title": title,
            "format": file_format,
            "chapter_count": str(chapter_count),
            "total_words": str(total_words),
            "line_count": str(line_count),
            "created_at": now.isoformat(),
        })

        # 批量写入小说章节内容表 (novel_contents.csv)
        chapter_records = []
        for idx, (ch_title, ch_content) in enumerate(chapters):
            chapter_records.append({
                "chapter_id": uuid.uuid4().hex,
                "novel_id": file_id,
                "chapter_index": str(idx),
                "chapter_title": ch_title,
                "content": ch_content,
                "word_count": str(count_words(ch_content)),
            })
        self.contents_table.insert_batch(chapter_records)

        return info

    def to_record(self, info: TextFileInfo) -> dict:
        """将领域模型转换为索引记录。"""
        parsed_title, _, _ = parse_novel_filename(info.filename)
        title = parsed_title or Path(info.filename).stem
        file_format = info.extension.lstrip(".")
        return {
            "novel_id": info.file_id,
            "title": title,
            "format": file_format,
            "chapter_count": "1",
            "total_words": str(info.size),
            "line_count": "1",
            "created_at": info.uploaded_at.isoformat(),
        }

    def record_to_info(self, record: dict) -> TextFileInfo:
        """将小说记录转换为领域模型。"""
        novel_id = record.get("novel_id") or record.get("file_id") or record.get("id") or ""
        title = record.get("title", "")
        file_format = record.get("format", "txt")
        extension = f".{file_format}" if file_format else ".txt"

        # 寻找磁盘上的实际物理文件
        stored_name = f"{novel_id}{extension}"
        size = int(record.get("total_words", 0) or 0)

        matches = list(self.upload_dir.glob(f"{novel_id}.*"))
        if matches:
            stored_name = matches[0].name
            extension = matches[0].suffix.lower()
            try:
                size = matches[0].stat().st_size
            except OSError:
                pass

        filename = f"{title}{extension}" if title else stored_name
        created_at_raw = record.get("created_at") or record.get("uploaded_at")
        uploaded_at = datetime.fromisoformat(created_at_raw) if created_at_raw else datetime.now(UTC)

        return TextFileInfo(
            file_id=novel_id,
            filename=filename,
            stored_name=stored_name,
            extension=extension,
            size=size,
            content_type=self.resolve_content_type(extension, "text/plain"),
            category=categorize(extension),
            uploaded_at=uploaded_at,
            download_url=f"/api/v1/text-files/{novel_id}/download",
            description=None,
        )

    def get_file(self, file_id: str) -> TextFileInfo:
        """根据 file_id 获取小说元数据。"""
        record = self.novels_table.find_by("novel_id", file_id)
        if record is None:
            raise FileNotFoundError(f"文件不存在：{file_id}")
        return self.record_to_info(record)

    def list_files(self, page: int = 1, page_size: int = 20) -> tuple[list[TextFileInfo], int]:
        """分页列出已上传小说，按创建时间倒序。"""
        rows = self.novels_table.find_all()
        items = [self.record_to_info(record) for record in rows]
        items.sort(key=lambda item: item.uploaded_at, reverse=True)

        total = len(items)
        start = (page - 1) * page_size
        return items[start : start + page_size], total

    def delete_file(self, file_id: str) -> TextFileInfo:
        """删除小说文件、元信息及其章节内容。"""
        info = self.get_file(file_id)

        target = self.upload_dir / info.stored_name
        try:
            target.unlink(missing_ok=True)
        except OSError as exc:
            raise StorageError(f"删除文件失败：{exc}") from exc

        self.novels_table.delete_by("novel_id", file_id)
        self.contents_table.delete_by("novel_id", file_id)
        return info

    def open_file(self, file_id: str) -> tuple[Path, TextFileInfo]:
        """打开文件用于下载，返回 (路径, 元数据)。"""
        info = self.get_file(file_id)
        path = self.upload_dir / info.stored_name
        if not path.exists():
            raise FileNotFoundError(f"文件不存在：{file_id}")
        return path, info

    def read_text(self, file_id: str, encoding: str = "utf-8") -> tuple[str, TextFileInfo]:
        """读取文本文件内容，返回 (文本, 元数据)。

        优先从物理文件读取；若物理文件已迁移则根据章节内容表拼接。
        """
        path, info = self.open_file(file_id)
        if not self.is_previewable(info.extension):
            raise InvalidParameterError(
                f"{info.extension} 为二进制文档格式，不支持文本内容预览，请使用下载接口"
            )

        if path.exists():
            try:
                return path.read_text(encoding=encoding), info
            except UnicodeDecodeError as exc:
                raise InvalidParameterError(f"文件不是合法的 {encoding} 文本：{exc}") from exc
            except OSError as exc:
                raise StorageError(f"读取文件失败：{exc}") from exc

        # 回退：从章节内容表中读取并拼接
        chapter_rows = self.contents_table.find_all(lambda r: r.get("novel_id") == file_id)
        if not chapter_rows:
            raise FileNotFoundError(f"文件不存在：{file_id}")

        text = "\n\n".join(
            f"{r['chapter_title']}\n{r['content']}" if r.get("chapter_title") != "全文" else r["content"]
            for r in chapter_rows
        )
        return text, info

    def get_novel_chapters(self, file_id: str) -> list[dict[str, str]]:
        """获取指定小说的全部章节记录。"""
        return self.contents_table.find_all(lambda r: r.get("novel_id") == file_id)

    def resolve_download(self, filename: str) -> tuple[Path, TextFileInfo]:
        """按原始文件名或书名解析下载目标（便于外部链接下载）。"""
        safe_name = self.sanitize_filename(filename)
        title = Path(safe_name).stem
        for row in self.novels_table.find_all():
            fmt = row.get("format", "txt")
            if (
                row.get("title") == title
                or f"{row.get('title')}.{fmt}" == safe_name
                or f"{row.get('title')}.txt" == safe_name
                or f"{row.get('title')}" == safe_name
            ):
                info = self.record_to_info(row)
                path = self.upload_dir / info.stored_name
                if not path.exists():
                    raise FileNotFoundError(f"文件不存在：{filename}")
                return path, info
        raise FileNotFoundError(f"文件不存在：{filename}")

    def cleanup(self) -> None:
        """清理上传目录与 CSV 表（测试或运维使用）。"""
        shutil.rmtree(self.upload_dir, ignore_errors=True)
        self.upload_dir.mkdir(parents=True, exist_ok=True)
        self.novels_table.clear()
        self.contents_table.clear()

    def iter_stored_names(self) -> Iterable[str]:
        """迭代所有已存储文件名。"""
        for record in self.novels_table.find_all():
            novel_id = record.get("novel_id")
            if novel_id:
                yield f"{novel_id}.txt"
