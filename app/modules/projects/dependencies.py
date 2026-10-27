"""项目管理模块的依赖注入。"""

from __future__ import annotations

from app.modules.projects.service import ProjectService
from app.modules.text_files.dependencies import get_storage_service
from app.modules.text_files.service import TextFileStorageService


def get_project_service(
    storage: TextFileStorageService | None = None,
) -> ProjectService:
    """获取项目服务实例。"""
    return ProjectService(storage_service=storage or get_storage_service())
