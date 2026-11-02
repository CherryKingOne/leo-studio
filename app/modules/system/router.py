"""系统功能模块的路由。

提供健康检查与服务信息等运维接口。

- ``router``：挂载在 ``/api/v1/system`` 下，随业务接口统一带版本前缀
- ``root_router``：挂在应用根路径下，提供 ``/`` 与 ``/health``
"""

from __future__ import annotations

from fastapi import APIRouter

from app.shared.config import get_settings

settings = get_settings()


def health_result() -> dict:
    """构造健康检查结果。"""
    return {"status": "ok", "app": settings.app_name, "version": settings.app_version}


router = APIRouter(prefix="/system", tags=["系统"])

root_router = APIRouter(tags=["系统"])


@router.get("/health", summary="健康检查")
async def system_health() -> dict:
    """系统模块健康检查接口。"""
    return health_result()


@root_router.get("/health", summary="健康检查")
async def health() -> dict:
    """应用健康检查接口。"""
    return health_result()


@root_router.get("/", summary="服务信息")
async def root() -> dict:
    """返回服务基本信息与接口索引。"""
    return {
        "app": settings.app_name,
        "version": settings.app_version,
        "docs": "/docs",
        "health": "/health",
        "api_prefix": "/api/v1",
        "modules": ["text-files", "projects", "system"],
    }
