"""FastAPI 应用入口。

启动方式::

    uvicorn app.main:app --reload
    leo-studio            # 通过 pyproject 的脚本入口
"""

from __future__ import annotations

import logging

from fastapi import FastAPI, Request
from fastapi.exceptions import RequestValidationError
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import JSONResponse
from starlette.exceptions import HTTPException as StarletteHTTPException

from app.router import api_router, system_root_router
from app.shared.config import get_settings
from app.shared.errors import AppError, ErrorCode
from app.shared.middleware import RequestIdMiddleware

logger = logging.getLogger(__name__)
settings = get_settings()

app = FastAPI(
    title=settings.app_name,
    version=settings.app_version,
    description=(
        "Leo Studio 后端服务。"
        "按业务功能划分为独立模块，当前提供文本文件的上传、预览、下载与管理能力。"
    ),
    debug=settings.debug,
    docs_url="/docs",
    redoc_url="/redoc",
    openapi_url="/openapi.json",
)

app.add_middleware(RequestIdMiddleware)
app.add_middleware(
    CORSMiddleware,
    allow_origins=settings.cors_origins,
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)


def _request_id(request: Request) -> str | None:
    """从请求状态中取出 request_id。"""
    return getattr(request.state, "request_id", None)


@app.exception_handler(AppError)
async def app_error_handler(request: Request, exc: AppError) -> JSONResponse:
    """业务异常统一处理。"""
    logger.warning("业务异常: code=%s message=%s", exc.code, exc.message)
    return JSONResponse(
        status_code=exc.http_status,
        content={
            "code": exc.code,
            "message": exc.message,
            "data": exc.details,
            "request_id": _request_id(request),
        },
    )


@app.exception_handler(RequestValidationError)
async def validation_error_handler(request: Request, exc: RequestValidationError) -> JSONResponse:
    """请求参数校验失败统一处理。"""
    return JSONResponse(
        status_code=422,
        content={
            "code": ErrorCode.INVALID_PARAMETER,
            "message": "请求参数校验失败",
            "data": exc.errors(),
            "request_id": _request_id(request),
        },
    )


@app.exception_handler(StarletteHTTPException)
async def http_error_handler(request: Request, exc: StarletteHTTPException) -> JSONResponse:
    """HTTP 异常统一处理。"""
    return JSONResponse(
        status_code=exc.status_code,
        content={
            "code": exc.status_code * 100,
            "message": str(exc.detail),
            "data": None,
            "request_id": _request_id(request),
        },
    )


@app.exception_handler(Exception)
async def unhandled_error_handler(request: Request, exc: Exception) -> JSONResponse:
    """未捕获异常统一处理，避免泄露内部细节。"""
    logger.exception("未处理异常: %s", exc)
    return JSONResponse(
        status_code=500,
        content={
            "code": ErrorCode.INTERNAL_ERROR,
            "message": "服务器内部错误",
            "data": None,
            "request_id": _request_id(request),
        },
    )


app.include_router(api_router)
app.include_router(system_root_router)


def run() -> None:
    """以 uvicorn 启动开发服务器。"""
    import uvicorn

    uvicorn.run("app.main:app", host="0.0.0.0", port=8000, reload=settings.debug)
