"""API 路由汇总。

按功能模块聚合各模块的路由。新增功能模块时，
只需在此处注册对应模块的 router 即可，模块之间互不感知。
"""

from fastapi import APIRouter

from app.modules.system.router import root_router as system_root_router
from app.modules.system.router import router as system_router
from app.modules.text_files.router import router as text_files_router

api_router = APIRouter(prefix="/api/v1")
api_router.include_router(text_files_router)
api_router.include_router(system_router)

__all__ = ["api_router", "system_root_router"]
