"""应用配置。

所有配置项均可通过环境变量或 ``.env`` 文件覆盖，环境变量前缀为 ``LEO_``，
例如 ``LEO_MAX_UPLOAD_SIZE_MB`` 覆盖全局上传大小上限。

各功能模块可拥有自己的独立配置（见 ``app/modules/*/config.py``），
本文件只保留跨模块通用的应用级配置。
"""

from __future__ import annotations

from functools import lru_cache

from pydantic import Field, field_validator
from pydantic_settings import BaseSettings, SettingsConfigDict


class Settings(BaseSettings):
    """应用运行期配置。"""

    model_config = SettingsConfigDict(
        env_prefix="LEO_",
        env_file=".env",
        env_file_encoding="utf-8",
        extra="ignore",
    )

    app_name: str = "Leo Studio API"
    app_version: str = "0.1.0"
    debug: bool = True

    # 允许跨域访问的来源，"*" 表示允许全部。
    cors_origins: list[str] = Field(default_factory=lambda: ["*"])

    # 允许上传的最大文件大小，单位 MB。
    # 各功能模块可通过自己的配置覆盖此上限。
    max_upload_size_mb: int = 50

    @field_validator("cors_origins", mode="before")
    @classmethod
    def _split_csv(cls, value: object) -> object:
        """允许使用逗号分隔的字符串形式配置列表。"""
        if isinstance(value, str):
            return [item.strip() for item in value.split(",") if item.strip()]
        return value

    @property
    def max_upload_size_bytes(self) -> int:
        """最大上传文件大小（字节）。"""
        return self.max_upload_size_mb * 1024 * 1024


@lru_cache(maxsize=1)
def get_settings() -> Settings:
    """返回进程内缓存的配置单例。"""
    return Settings()


settings = get_settings()


__all__ = ["Settings", "get_settings", "settings"]
