"""文本文件模块的独立配置。

与全局配置分离，使每个功能模块可以拥有自己的白名单、限制等规则，
新增功能模块时互不影响。环境变量前缀为 ``LEO_TEXT_FILES_``。
"""

from __future__ import annotations

from functools import lru_cache
from pathlib import Path

from pydantic import Field, field_validator
from pydantic_settings import BaseSettings, SettingsConfigDict

from app.modules.text_files.constants import TEXT_FILE_EXTENSIONS


class TextFilesSettings(BaseSettings):
    """文本文件功能模块的运行期配置。"""

    model_config = SettingsConfigDict(
        env_prefix="LEO_TEXT_FILES_",
        env_file=".env",
        env_file_encoding="utf-8",
        extra="ignore",
    )

    # 允许上传的文本文件扩展名白名单，为空表示不限制。
    allowed_extensions: list[str] = Field(default_factory=lambda: sorted(TEXT_FILE_EXTENSIONS))
    # 单个文本文件大小上限，单位 MB；为 None 时沿用全局限制。
    max_upload_size_mb: int | None = None
    # 文本文件存储目录，相对路径基于项目根目录解析。
    upload_dir: Path = Path("storage/uploads/text-files")
    # 文件名最大长度。
    max_filename_length: int = 255

    @field_validator("allowed_extensions", mode="before")
    @classmethod
    def split_csv(cls, value: object) -> object:
        """允许使用逗号分隔的字符串形式配置列表。"""
        if isinstance(value, str):
            return [item.strip() for item in value.split(",") if item.strip()]
        return value

    @field_validator("upload_dir", mode="after")
    @classmethod
    def resolve_upload_dir(cls, value: Path) -> Path:
        """将相对路径转换为基于项目根目录的绝对路径。"""
        if value.is_absolute():
            return value
        return (Path(__file__).resolve().parents[3] / value).resolve()


@lru_cache(maxsize=1)
def get_text_files_settings() -> TextFilesSettings:
    """返回进程内缓存的模块配置单例。"""
    return TextFilesSettings()
