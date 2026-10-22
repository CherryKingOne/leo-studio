"""统一错误定义与业务异常。

所有接口使用同一套响应包装结构：

.. code-block:: json

    {
        "code": 0,
        "message": "success",
        "data": {},
        "request_id": "0f3c..."
    }

``code`` 为 ``0`` 表示成功，非 ``0`` 表示业务或系统错误。
"""

from __future__ import annotations

from typing import Any


class ErrorCode:
    """业务错误码常量。"""

    SUCCESS = 0
    INVALID_PARAMETER = 40000
    FILE_TOO_LARGE = 41300
    UNSUPPORTED_FILE_TYPE = 41500
    FILE_NOT_FOUND = 40400
    FILE_CONFLICT = 40900
    STORAGE_ERROR = 50000
    INTERNAL_ERROR = 50001


class AppError(Exception):
    """业务异常基类，由全局异常处理器转换为标准响应。"""

    code: int = ErrorCode.INTERNAL_ERROR
    message: str = "服务器内部错误"
    http_status: int = 500

    def __init__(
        self,
        message: str | None = None,
        *,
        code: int | None = None,
        http_status: int | None = None,
        details: Any = None,
    ) -> None:
        self.message = message or self.message
        self.code = code if code is not None else self.code
        self.http_status = http_status if http_status is not None else self.http_status
        self.details = details
        super().__init__(self.message)


class InvalidParameterError(AppError):
    """请求参数不合法。"""

    code = ErrorCode.INVALID_PARAMETER
    message = "请求参数不合法"
    http_status = 422


class FileTooLargeError(AppError):
    """上传文件超过大小限制。"""

    code = ErrorCode.FILE_TOO_LARGE
    message = "上传文件超过大小限制"
    http_status = 413


class UnsupportedFileTypeError(AppError):
    """上传的文件类型不被允许。"""

    code = ErrorCode.UNSUPPORTED_FILE_TYPE
    message = "不支持的文件类型"
    http_status = 415


class FileNotFoundError(AppError):
    """目标文件不存在。"""

    code = ErrorCode.FILE_NOT_FOUND
    message = "文件不存在"
    http_status = 404


class FileConflictError(AppError):
    """目标文件已存在。"""

    code = ErrorCode.FILE_CONFLICT
    message = "文件已存在"
    http_status = 409


class StorageError(AppError):
    """文件系统操作失败。"""

    code = ErrorCode.STORAGE_ERROR
    message = "文件存储失败"
    http_status = 500
