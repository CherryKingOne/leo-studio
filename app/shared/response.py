"""统一响应结构与构造工具。"""

from __future__ import annotations

from contextvars import ContextVar
from typing import Any

from pydantic import BaseModel, Field, model_validator

from app.shared.errors import ErrorCode

#: 当前请求的追踪 ID，由中间件在请求进入时写入，供响应模型自动填充。
_request_id_var: ContextVar[str | None] = ContextVar("request_id", default=None)


def set_request_id(value: str | None) -> None:
    """设置当前上下文的请求追踪 ID。"""
    _request_id_var.set(value)


def get_request_id() -> str | None:
    """读取当前上下文的请求追踪 ID。"""
    return _request_id_var.get()


class ApiResponse[T](BaseModel):
    """统一响应包装。"""

    code: int = Field(default=ErrorCode.SUCCESS, description="业务状态码，0 表示成功")
    message: str = Field(default="success", description="提示信息")
    data: T | None = Field(default=None, description="业务数据")
    request_id: str | None = Field(default=None, description="请求追踪 ID")

    @model_validator(mode="after")
    def _fill_request_id(self) -> ApiResponse[T]:
        """未显式指定 request_id 时，从请求上下文自动填充。"""
        if self.request_id is None:
            object.__setattr__(self, "request_id", get_request_id())
        return self


def success(data: Any = None, message: str = "success") -> dict[str, Any]:
    """构造成功响应字典。"""
    return {"code": ErrorCode.SUCCESS, "message": message, "data": data}


def failure(code: int, message: str, data: Any = None) -> dict[str, Any]:
    """构造失败响应字典。"""
    return {"code": code, "message": message, "data": data}
