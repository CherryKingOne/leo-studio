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
]

#: 项目历史快照版本 CSV 持久化文件名
PROJECT_VERSIONS_CSV_FILENAME: Final[str] = "project_versions.csv"

#: 项目版本表表头字段
PROJECT_VERSIONS_COLUMNS: Final[list[str]] = [
    "version_id",
    "project_id",
    "file_id",
    "version_number",
    "filename",
    "chapter_count",
    "total_words",
    "created_at",
    "remark",
]

#: 单个项目最大保留的历史快照版本数量（滑动窗口淘汰上限）
MAX_PROJECT_VERSIONS: Final[int] = 3

