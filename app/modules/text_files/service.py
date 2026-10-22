"""文本文件存储服务：负责落盘、校验、元数据索引与读取。

设计要点：

- 文件以 ``file_id`` 作为唯一标识，原始文件名保存在索引中，
  实际落盘文件名形如 ``{file_id}{ext}``，避免重名覆盖与路径穿越。
- 扩展名白名单来自模块自身配置，文本文件之外的类型一律拒绝。
- 批量保存采用“全成功或全回滚”语义，任一文件失败则清理本次已落盘的文件。
"""

from __future__ import annotations

import json
import re
import shutil
import uuid
from collections.abc import Iterable
from datetime import UTC, datetime
from pathlib import Path
from typing import BinaryIO

from app.modules.text_files.config import TextFilesSettings, get_text_files_settings
from app.modules.text_files.constants import BINARY_EXTENSIONS, EXTENSION_MIME_MAP, INDEX_FILENAME
from app.modules.text_files.models import TextFileInfo, categorize
from app.shared.config import get_settings
from app.shared.errors import (
    FileNotFoundError,
    FileTooLargeError,
    InvalidParameterError,
    StorageError,
    UnsupportedFileTypeError,
)

SAFE_NAME_RE = re.compile(r"[^\w一-鿿\-.]")


def human_size(size: int) -> str:
    """将字节数转换为可读字符串。"""
    value = float(size)
    for unit in ("B", "KB", "MB", "GB", "TB"):
        if value < 1024:
            return f"{int(value)} B" if unit == "B" else f"{value:.2f} {unit}"
        value /= 1024
    return f"{value:.2f} TB"


class TextFileStorageService:
    """文本文件的本地文件系统存储实现。"""

    def __init__(self, settings: TextFilesSettings | None = None) -> None:
        self.settings = settings or get_text_files_settings()
        self.upload_dir = Path(self.settings.upload_dir)
        self.upload_dir.mkdir(parents=True, exist_ok=True)
        self.index_path = self.upload_dir / INDEX_FILENAME

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
        """确定文件的 MIME 类型。

        扩展名是白名单的唯一准入依据，因此对已知扩展名以扩展名推导的 MIME 为准，
        避免二进制文档被标记成 ``text/plain`` 或 ``application/octet-stream``；
        仅当扩展名不在映射表内（如通过配置追加的自定义扩展名）时，
        才沿用客户端提交的值。
        """
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
    # 索引读写
    # ------------------------------------------------------------------
    def load_index(self) -> dict[str, dict]:
        if not self.index_path.exists():
            return {}
        try:
            raw = json.loads(self.index_path.read_text(encoding="utf-8"))
        except (json.JSONDecodeError, OSError) as exc:
            raise StorageError(f"读取文件索引失败：{exc}") from exc
        return raw if isinstance(raw, dict) else {}

    def save_index(self, index: dict[str, dict]) -> None:
        try:
            self.index_path.write_text(
                json.dumps(index, ensure_ascii=False, indent=2), encoding="utf-8"
            )
        except OSError as exc:
            raise StorageError(f"写入文件索引失败：{exc}") from exc

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
        """保存单个文本文件并返回元数据。

        写入时按块读取并实时校验大小，超限立即中断并删除半成品文件。
        """
        safe_name = self.sanitize_filename(filename)
        extension = Path(safe_name).suffix.lower()
        self.check_extension(extension)

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

        info = TextFileInfo(
            file_id=file_id,
            filename=safe_name,
            stored_name=target.name,
            extension=extension,
            size=written,
            content_type=self.resolve_content_type(extension, content_type),
            category=categorize(extension),
            uploaded_at=datetime.now(UTC),
            download_url=f"/api/v1/text-files/{file_id}/download",
            description=description,
        )

        index = self.load_index()
        index[file_id] = self.to_record(info)
        self.save_index(index)
        return info

    def save_files(
        self,
        items: Iterable[tuple[str, BinaryIO, str]],
        *,
        descriptions: Iterable[str | None] | None = None,
    ) -> list[TextFileInfo]:
        """批量保存文本文件，采用全成功或全回滚语义。

        任一文件保存失败时，本次调用已落盘的文件与索引记录会被清理，
        不会留下半成品数据。
        """
        item_list = list(items)
        description_list = list(descriptions) if descriptions is not None else []
        saved: list[TextFileInfo] = []
        try:
            for index, (filename, content, content_type) in enumerate(item_list):
                description = description_list[index] if index < len(description_list) else None
                saved.append(
                    self.save_file(
                        filename,
                        content,
                        content_type=content_type,
                        description=description,
                    )
                )
        except Exception:
            self.rollback(saved)
            raise
        return saved

    def rollback(self, infos: list[TextFileInfo]) -> None:
        """回滚批量保存：删除已落盘文件并清理索引记录。"""
        if not infos:
            return
        for info in infos:
            (self.upload_dir / info.stored_name).unlink(missing_ok=True)
        index = self.load_index()
        changed = False
        for info in infos:
            if index.pop(info.file_id, None) is not None:
                changed = True
        if changed:
            self.save_index(index)

    def to_record(self, info: TextFileInfo) -> dict:
        """将领域模型转换为索引记录。"""
        return {
            "file_id": info.file_id,
            "filename": info.filename,
            "stored_name": info.stored_name,
            "extension": info.extension,
            "size": info.size,
            "content_type": info.content_type,
            "category": str(info.category),
            "uploaded_at": info.uploaded_at.isoformat(),
            "description": info.description,
        }

    def record_to_info(self, record: dict) -> TextFileInfo:
        """将索引记录转换为领域模型。"""
        file_id = record["file_id"]
        return TextFileInfo(
            file_id=file_id,
            filename=record["filename"],
            stored_name=record["stored_name"],
            extension=record.get("extension", ""),
            size=int(record.get("size", 0)),
            content_type=record.get("content_type", "text/plain"),
            category=categorize(record.get("extension", "")),
            uploaded_at=datetime.fromisoformat(record["uploaded_at"]),
            download_url=f"/api/v1/text-files/{file_id}/download",
            description=record.get("description"),
        )

    def get_file(self, file_id: str) -> TextFileInfo:
        """根据 file_id 获取文件元数据。"""
        index = self.load_index()
        record = index.get(file_id)
        if record is None:
            raise FileNotFoundError(f"文件不存在：{file_id}")
        return self.record_to_info(record)

    def list_files(self, page: int = 1, page_size: int = 20) -> tuple[list[TextFileInfo], int]:
        """分页列出已上传文件，按上传时间倒序。"""
        index = self.load_index()
        items = [self.record_to_info(record) for record in index.values()]
        items.sort(key=lambda item: item.uploaded_at, reverse=True)

        total = len(items)
        start = (page - 1) * page_size
        return items[start : start + page_size], total

    def delete_file(self, file_id: str) -> TextFileInfo:
        """删除文件及其索引记录。"""
        index = self.load_index()
        record = index.get(file_id)
        if record is None:
            raise FileNotFoundError(f"文件不存在：{file_id}")

        target = self.upload_dir / record["stored_name"]
        try:
            target.unlink(missing_ok=True)
        except OSError as exc:
            raise StorageError(f"删除文件失败：{exc}") from exc

        del index[file_id]
        self.save_index(index)
        return self.record_to_info(record)

    def open_file(self, file_id: str) -> tuple[Path, TextFileInfo]:
        """打开文件用于下载，返回 (路径, 元数据)。"""
        info = self.get_file(file_id)
        path = self.upload_dir / info.stored_name
        if not path.exists():
            raise FileNotFoundError(f"文件不存在：{file_id}")
        return path, info

    def read_text(self, file_id: str, encoding: str = "utf-8") -> tuple[str, TextFileInfo]:
        """读取文本文件内容，返回 (文本, 元数据)。

        仅纯文本类型可预览；二进制文档（Word / EPUB）需通过下载接口获取。
        """
        path, info = self.open_file(file_id)
        if not self.is_previewable(info.extension):
            raise InvalidParameterError(
                f"{info.extension} 为二进制文档格式，不支持文本内容预览，请使用下载接口"
            )
        try:
            text = path.read_text(encoding=encoding)
        except UnicodeDecodeError as exc:
            raise InvalidParameterError(f"文件不是合法的 {encoding} 文本：{exc}") from exc
        except OSError as exc:
            raise StorageError(f"读取文件失败：{exc}") from exc
        return text, info

    def resolve_download(self, filename: str) -> tuple[Path, TextFileInfo]:
        """按原始文件名解析下载目标（便于外部链接下载）。"""
        index = self.load_index()
        safe_name = self.sanitize_filename(filename)
        for record in index.values():
            if record["filename"] == safe_name:
                info = self.record_to_info(record)
                path = self.upload_dir / info.stored_name
                if not path.exists():
                    raise FileNotFoundError(f"文件不存在：{filename}")
                return path, info
        raise FileNotFoundError(f"文件不存在：{filename}")

    def cleanup(self) -> None:
        """清理上传目录（测试或运维使用）。"""
        shutil.rmtree(self.upload_dir, ignore_errors=True)
        self.upload_dir.mkdir(parents=True, exist_ok=True)

    def iter_stored_names(self) -> Iterable[str]:
        """迭代所有已存储文件名。"""
        for record in self.load_index().values():
            yield record["stored_name"]
