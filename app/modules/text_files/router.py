"""文本文件上传与下载路由。

所有接口前缀：``/api/v1/text-files``

本模块只处理文本类文件。若后续需要支持图片、视频等二进制类型，
请新建对应的功能模块（``app/modules/xxx``）并注册独立路由，
不要在本模块中混合处理。
"""

from __future__ import annotations

from typing import Annotated

from fastapi import APIRouter, Depends, File, Form, Path, Query, UploadFile
from fastapi.responses import FileResponse

from app.modules.text_files.constants import MODULE_TAG, ROUTE_PREFIX
from app.modules.text_files.dependencies import get_storage_service
from app.modules.text_files.models import TextFileInfo
from app.modules.text_files.schemas import (
    TextFileContentResponse,
    TextFileDeleteResponse,
    TextFileListItem,
    TextFileListResponse,
    TextFileUploadResponse,
)
from app.modules.text_files.service import TextFileStorageService, human_size
from app.shared.errors import InvalidParameterError
from app.shared.response import ApiResponse

router = APIRouter(prefix=ROUTE_PREFIX, tags=[MODULE_TAG])

FileIdPath = Annotated[
    str,
    Path(description="文件唯一标识，32 位十六进制字符串"),
]


def to_upload_response(info: TextFileInfo) -> TextFileUploadResponse:
    """将领域模型转换为上传响应 Schema。"""
    return TextFileUploadResponse(
        file_id=info.file_id,
        filename=info.filename,
        stored_name=info.stored_name,
        extension=info.extension,
        size=info.size,
        size_human=human_size(info.size),
        content_type=info.content_type,
        category=info.category,
        uploaded_at=info.uploaded_at,
        download_url=info.download_url,
        description=info.description,
    )


@router.post(
    "/upload",
    response_model=ApiResponse[TextFileUploadResponse],
    summary="上传单个文本文件",
    description=(
        "以 ``multipart/form-data`` 方式上传单个文本文件。\n\n"
        "- 表单字段名固定为 `file`\n"
        "- 可选字段 `description` 用于记录文件说明\n"
        "- 仅接受 `.txt` `.md` `.doc` `.docx` `.epub` 五种扩展名\n"
        "- 返回 `file_id` 可用于后续下载、预览或删除"
    ),
)
async def upload_text_file(
    file: Annotated[UploadFile, File(description="待上传的文本文件")],
    description: Annotated[str | None, Form(description="文件描述")] = None,
    storage: TextFileStorageService = Depends(get_storage_service),
) -> ApiResponse[TextFileUploadResponse]:
    """上传单个文本文件。"""
    if file.filename is None or not file.filename.strip():
        raise InvalidParameterError("文件名不能为空")

    info = storage.save_file(
        filename=file.filename,
        content=file.file,
        content_type=file.content_type or "text/plain",
        description=description,
    )
    return ApiResponse(data=to_upload_response(info), message="文件上传成功")


@router.get(
    "",
    response_model=ApiResponse[TextFileListResponse],
    summary="分页查询文本文件列表",
    description="按上传时间倒序返回文本文件列表，支持分页。",
)
async def list_text_files(
    page: Annotated[int, Query(ge=1, description="页码，从 1 开始")] = 1,
    page_size: Annotated[int, Query(ge=1, le=100, description="每页数量，最大 100")] = 20,
    storage: TextFileStorageService = Depends(get_storage_service),
) -> ApiResponse[TextFileListResponse]:
    """分页查询已上传的文本文件。"""
    items, total = storage.list_files(page=page, page_size=page_size)
    pages = (total + page_size - 1) // page_size if total else 0
    return ApiResponse(
        data=TextFileListResponse(
            total=total,
            page=page,
            page_size=page_size,
            pages=pages,
            items=[
                TextFileListItem(
                    file_id=item.file_id,
                    filename=item.filename,
                    extension=item.extension,
                    size=item.size,
                    size_human=human_size(item.size),
                    category=item.category,
                    uploaded_at=item.uploaded_at,
                    download_url=item.download_url,
                )
                for item in items
            ],
        )
    )


@router.get(
    "/{file_id}",
    response_model=ApiResponse[TextFileUploadResponse],
    summary="查询文本文件详情",
    description="根据 `file_id` 查询文本文件的完整元信息。",
)
async def get_text_file(
    file_id: FileIdPath,
    storage: TextFileStorageService = Depends(get_storage_service),
) -> ApiResponse[TextFileUploadResponse]:
    """查询文本文件详情。"""
    info = storage.get_file(file_id)
    return ApiResponse(data=to_upload_response(info))


@router.get(
    "/{file_id}/content",
    response_model=ApiResponse[TextFileContentResponse],
    summary="预览文本文件内容",
    description=(
        "根据 `file_id` 读取文本文件内容并以 JSON 返回，适合在线预览。\n\n"
        "- 默认按 UTF-8 解码\n"
        "- 可通过 `encoding` 参数指定其它字符集\n"
        "- 仅 `.txt` `.md` 支持预览；`.doc` `.docx` `.epub` 为二进制文档，"
        "调用本接口会返回 `422`，请改用下载接口"
    ),
)
async def get_text_file_content(
    file_id: FileIdPath,
    encoding: Annotated[str, Query(description="解码字符集，默认 utf-8")] = "utf-8",
    storage: TextFileStorageService = Depends(get_storage_service),
) -> ApiResponse[TextFileContentResponse]:
    """读取文本文件内容。"""
    text, info = storage.read_text(file_id, encoding=encoding)
    return ApiResponse(
        data=TextFileContentResponse(
            file_id=info.file_id,
            filename=info.filename,
            content_type=info.content_type,
            encoding=encoding,
            size=info.size,
            content=text,
        )
    )


@router.get(
    "/{file_id}/download",
    summary="下载文本文件",
    description="根据 `file_id` 下载文件内容，响应头携带原始文件名。",
    response_class=FileResponse,
)
async def download_text_file(
    file_id: FileIdPath,
    storage: TextFileStorageService = Depends(get_storage_service),
) -> FileResponse:
    """下载文本文件内容。"""
    path, info = storage.open_file(file_id)
    return FileResponse(
        path=path,
        media_type=info.content_type,
        filename=info.filename,
    )


@router.delete(
    "/{file_id}",
    response_model=ApiResponse[TextFileDeleteResponse],
    summary="删除文本文件",
    description="根据 `file_id` 删除文件及其索引记录。",
)
async def delete_text_file(
    file_id: FileIdPath,
    storage: TextFileStorageService = Depends(get_storage_service),
) -> ApiResponse[TextFileDeleteResponse]:
    """删除文本文件。"""
    info = storage.delete_file(file_id)
    return ApiResponse(
        data=TextFileDeleteResponse(file_id=info.file_id, filename=info.filename, deleted=True),
        message="文件删除成功",
    )
