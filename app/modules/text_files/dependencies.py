"""文本文件模块的依赖注入提供者。

模块通过本文件暴露 FastAPI ``Depends`` 目标，
路由层不直接实例化服务，便于测试时整体替换。
"""

from __future__ import annotations

from functools import lru_cache

from app.modules.text_files.service import TextFileStorageService


@lru_cache(maxsize=1)
def get_storage_service() -> TextFileStorageService:
    """返回进程内缓存的文本文件存储服务单例。"""
    return TextFileStorageService()


__all__ = ["get_storage_service"]
