"""项目管理模块的常量与类型定义。"""

from __future__ import annotations

from typing import Final

#: 模块路由前缀，与 api_router 的 /api/v1 组合后为 /api/v1/projects
ROUTE_PREFIX: Final[str] = "/projects"

#: 模块在 OpenAPI 中的标签名
MODULE_TAG: Final[str] = "项目管理"

#: 项目 CSV 持久化文件名
PROJECTS_CSV_FILENAME: Final[str] = "projects.csv"

#: 项目表表头字段
PROJECTS_COLUMNS: Final[list[str]] = [
    "project_id",
    "name",
    "file_id",
    "filename",
    "status",
    "chapter_count",
    "total_words",
    "created_at",
    "updated_at",
    "is_deleted",
    "deleted_at",
]


