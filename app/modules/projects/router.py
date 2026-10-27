"""项目管理路由。

所有接口前缀：``/api/v1/projects``

提供项目的创建（绑定小说上传）、分页列表、详情查询与删除能力。
直接复用文本文件模块完成小说的清洗、分章与持久化。
"""

from __future__ import annotations

from typing import Annotated

from fastapi import APIRouter, Depends, File, Form, Path, Query, UploadFile

from app.modules.projects.constants import MODULE_TAG, ROUTE_PREFIX
from app.modules.projects.models import (
    ChapterConflictInfo,
    ChapterContentInfo,
    ChapterOutlineInfo,
    ProjectChapterAppendInfo,
    ProjectInfo,
    ProjectRollbackInfo,
    ProjectVersionInfo,
)
from app.modules.projects.schemas import (
    ChapterConflictItem,
    ChapterContentResponse,
    ChapterOutlineItem,
    ChapterOutlineListResponse,
    ChapterSentenceItem,
    ProjectChapterAppendResponse,
    ProjectDeleteResponse,
    ProjectListItem,
    ProjectListResponse,
    ProjectResponse,
    ProjectRollbackResponse,
    ProjectVersionItem,
    ProjectVersionListResponse,
)
from app.modules.projects.service import ProjectService
from app.modules.text_files.dependencies import get_storage_service
from app.modules.text_files.service import TextFileStorageService
from app.shared.errors import InvalidParameterError
from app.shared.response import ApiResponse

router = APIRouter(prefix=ROUTE_PREFIX, tags=[MODULE_TAG])

ProjectIdPath = Annotated[
    str,
    Path(description="项目唯一标识，UUID (UUID4)"),
]

ChapterIdPath = Annotated[
    str,
    Path(description="章节唯一标识，UUID (UUID4)"),
]

VersionIdPath = Annotated[
    str,
    Path(description="版本快照唯一标识，UUID (UUID4)"),
]



def resolve_project_service(
    storage: TextFileStorageService = Depends(get_storage_service),
) -> ProjectService:
    """按依赖注入生成 ProjectService 实例。"""
    return ProjectService(storage_service=storage)


def to_project_response(info: ProjectInfo) -> ProjectResponse:
    """将领域模型转换为项目响应 Schema。"""
    return ProjectResponse(
        project_id=info.project_id,
        name=info.name,
        file_id=info.file_id,
        filename=info.filename,
        status=info.status,
        chapter_count=info.chapter_count,
        total_words=info.total_words,
        created_at=info.created_at,
        updated_at=info.updated_at,
    )


def to_project_list_item(info: ProjectInfo) -> ProjectListItem:
    """将领域模型转换为项目列表项 Schema。"""
    return ProjectListItem(
        project_id=info.project_id,
        name=info.name,
        file_id=info.file_id,
        filename=info.filename,
        status=info.status,
        chapter_count=info.chapter_count,
        total_words=info.total_words,
        created_at=info.created_at,
        updated_at=info.updated_at,
    )


def to_chapter_outline_item(info: ChapterOutlineInfo) -> ChapterOutlineItem:
    """将领域模型转换为章节大纲项 Schema。"""
    return ChapterOutlineItem(
        chapter_id=info.chapter_id,
        chapter_index=info.chapter_index,
        chapter_title=info.chapter_title,
        word_count=info.word_count,
    )


def to_chapter_content_response(info: ChapterContentInfo) -> ChapterContentResponse:
    """将领域模型转换为章节详细内容响应 Schema。"""
    return ChapterContentResponse(
        project_id=info.project_id,
        file_id=info.file_id,
        chapter_id=info.chapter_id,
        chapter_index=info.chapter_index,
        chapter_title=info.chapter_title,
        word_count=info.word_count,
        content=info.content,
        sentences=[
            ChapterSentenceItem(
                sentence_id=s.sentence_id,
                sentence_index=s.sentence_index,
                content=s.content,
                word_count=s.word_count,
            )
            for s in info.sentences
        ],
    )


def to_chapter_conflict_item(info: ChapterConflictInfo) -> ChapterConflictItem:
    """将领域模型转换为章节冲突项 Schema。"""
    return ChapterConflictItem(
        chapter_id=info.chapter_id,
        chapter_index=info.chapter_index,
        chapter_title=info.chapter_title,
        existing_word_count=info.existing_word_count,
        incoming_word_count=info.incoming_word_count,
        word_diff=info.word_diff,
    )


def to_chapter_append_response(
    info: ProjectChapterAppendInfo,
) -> ProjectChapterAppendResponse:
    """将领域模型转换为追加章节响应 Schema。"""
    return ProjectChapterAppendResponse(
        project_id=info.project_id,
        file_id=info.file_id,
        version_id=info.version_id,
        version_number=info.version_number,
        added_chapters=info.added_chapters,
        skipped_chapters=info.skipped_chapters,
        total_chapters=info.total_chapters,
        total_words=info.total_words,
        has_conflicts=info.has_conflicts,
        conflicts=[to_chapter_conflict_item(c) for c in info.conflicts],
        updated_at=info.updated_at,
    )


def to_project_version_item(info: ProjectVersionInfo) -> ProjectVersionItem:
    """将领域模型转换为历史版本项 Schema。"""
    return ProjectVersionItem(
        version_id=info.version_id,
        project_id=info.project_id,
        file_id=info.file_id,
        version_number=info.version_number,
        filename=info.filename,
        chapter_count=info.chapter_count,
        total_words=info.total_words,
        created_at=info.created_at,
        remark=info.remark,
    )


def to_project_rollback_response(
    info: ProjectRollbackInfo,
) -> ProjectRollbackResponse:
    """将领域模型转换为回滚响应 Schema。"""
    return ProjectRollbackResponse(
        project_id=info.project_id,
        restored_version_id=info.restored_version_id,
        restored_version_number=info.restored_version_number,
        chapter_count=info.chapter_count,
        total_words=info.total_words,
        updated_at=info.updated_at,
    )




@router.post(
    "",
    response_model=ApiResponse[ProjectResponse],
    summary="创建项目并上传小说",
    description=(
        "以 ``multipart/form-data`` 方式创建新项目并上传小说文件。\n\n"
        "- 请求参数：`name`（项目名称）与 `file`（上传文件）\n"
        "- 自动调用文本文件模块进行文件清洗、分章与切句持久化"
    ),
)
@router.post(
    "/upload",
    response_model=ApiResponse[ProjectResponse],
    summary="创建项目并上传小说（别名路径）",
    include_in_schema=False,
)
async def create_project(
    file: Annotated[UploadFile, File(description="上传文件")],
    name: Annotated[str, Form(description="项目名称")],
    service: ProjectService = Depends(resolve_project_service),
) -> ApiResponse[ProjectResponse]:
    """创建项目并上传小说文件。"""
    actual_name = name.strip()
    if not actual_name:
        raise InvalidParameterError("项目名称不能为空")

    if file.filename is None or not file.filename.strip():
        raise InvalidParameterError("上传文件不能为空")

    info = service.create_project(
        name=actual_name,
        filename=file.filename,
        content=file.file,
        content_type=file.content_type or "text/plain",
    )
    return ApiResponse(data=to_project_response(info), message="项目创建成功")


@router.get(
    "",
    response_model=ApiResponse[ProjectListResponse],
    summary="分页查询项目列表",
    description="按创建时间倒序返回项目列表，支持分页。",
)
async def list_projects(
    page: Annotated[int, Query(ge=1, description="页码，从 1 开始")] = 1,
    page_size: Annotated[int, Query(ge=1, le=100, description="每页数量，最大 100")] = 20,
    service: ProjectService = Depends(resolve_project_service),
) -> ApiResponse[ProjectListResponse]:
    """分页查询项目列表。"""
    items, total = service.list_projects(page=page, page_size=page_size)
    pages = (total + page_size - 1) // page_size if total else 0
    return ApiResponse(
        data=ProjectListResponse(
            total=total,
            page=page,
            page_size=page_size,
            pages=pages,
            items=[to_project_list_item(item) for item in items],
        )
    )


@router.get(
    "/{project_id}",
    response_model=ApiResponse[ProjectResponse],
    summary="查询项目详情",
    description="根据 `project_id` 查询单个项目的详细信息。",
)
async def get_project(
    project_id: ProjectIdPath,
    service: ProjectService = Depends(resolve_project_service),
) -> ApiResponse[ProjectResponse]:
    """查询项目详情。"""
    info = service.get_project(project_id)
    return ApiResponse(data=to_project_response(info))


@router.delete(
    "/{project_id}",
    response_model=ApiResponse[ProjectDeleteResponse],
    summary="删除项目",
    description="根据 `project_id` 删除项目及其关联的小说文件、章节与切句记录。",
)
async def delete_project(
    project_id: ProjectIdPath,
    service: ProjectService = Depends(resolve_project_service),
) -> ApiResponse[ProjectDeleteResponse]:
    """删除项目。"""
    info = service.delete_project(project_id)
    return ApiResponse(
        data=ProjectDeleteResponse(
            project_id=info.project_id,
            name=info.name,
            deleted=True,
        ),
        message="项目删除成功",
    )


@router.get(
    "/{project_id}/chapters",
    response_model=ApiResponse[ChapterOutlineListResponse],
    summary="查询项目章节大纲列表",
    description="获取指定项目下小说文件的章节大纲（目录列表），按章节序号升序排列。",
)
async def list_project_chapters(
    project_id: ProjectIdPath,
    service: ProjectService = Depends(resolve_project_service),
) -> ApiResponse[ChapterOutlineListResponse]:
    """查询项目章节大纲列表。"""
    items = service.get_project_chapters(project_id)
    return ApiResponse(
        data=ChapterOutlineListResponse(
            project_id=project_id,
            total=len(items),
            items=[to_chapter_outline_item(item) for item in items],
        )
    )


@router.get(
    "/{project_id}/chapters/{chapter_id}",
    response_model=ApiResponse[ChapterContentResponse],
    summary="查询项目章节详细内容",
    description="获取指定章节的完整正文内容及行级切句数据。",
)
@router.get(
    "/{project_id}/chapters/{chapter_id}/content",
    response_model=ApiResponse[ChapterContentResponse],
    summary="查询项目章节详细内容（别名路径）",
    include_in_schema=False,
)
async def get_project_chapter_content(
    project_id: ProjectIdPath,
    chapter_id: ChapterIdPath,
    service: ProjectService = Depends(resolve_project_service),
) -> ApiResponse[ChapterContentResponse]:
    """查询指定章节的详细内容与切句数据。"""
    info = service.get_project_chapter_content(project_id, chapter_id)
    return ApiResponse(data=to_chapter_content_response(info))


@router.post(
    "/{project_id}/chapters/upload",
    response_model=ApiResponse[ProjectChapterAppendResponse],
    summary="向项目追加上传章节文件",
    description=(
        "为已有项目增量上传小说文件，自动解析并追加新章节。\n\n"
        "- 请求路径：`project_id`\n"
        "- 表单参数：`file` 与可选的 `overwrite_existing`（默认 `false`）\n"
        "- 默认保护机制：非破坏性追加，自动跳过已有章节，防止劣质数据污染"
    ),

)
@router.post(
    "/{project_id}/chapters",
    response_model=ApiResponse[ProjectChapterAppendResponse],
    summary="向项目追加上传章节文件（别名路径）",
    include_in_schema=False,
)
async def append_project_chapters(
    project_id: ProjectIdPath,
    file: Annotated[UploadFile, File(description="增量上传的小说文件")],
    overwrite_existing: Annotated[
        bool, Form(description="是否覆盖已有章节（默认 false，仅增量追加）")
    ] = False,
    dry_run: Annotated[
        bool, Form(description="是否仅做差异预检，不修改实际数据（默认 false）")
    ] = False,
    service: ProjectService = Depends(resolve_project_service),
) -> ApiResponse[ProjectChapterAppendResponse]:
    """向项目增量追加章节文件。"""
    if file.filename is None or not file.filename.strip():
        raise InvalidParameterError("上传文件不能为空")

    info = service.append_chapters(
        project_id=project_id,
        filename=file.filename,
        content=file.file,
        content_type=file.content_type or "text/plain",
        overwrite_existing=overwrite_existing,
        dry_run=dry_run,
    )

    if dry_run:
        msg = (
            f"预检完成：预计新增 {info.added_chapters} 章，"
            f"跳过 {info.skipped_chapters} 章，冲突 {len(info.conflicts)} 处"
        )
    else:
        msg = f"章节追加成功，新增 {info.added_chapters} 章，跳过 {info.skipped_chapters} 章"

    return ApiResponse(
        data=to_chapter_append_response(info),
        message=msg,
    )


@router.get(
    "/{project_id}/versions",
    response_model=ApiResponse[ProjectVersionListResponse],
    summary="查询项目历史版本快照列表",
    description="获取指定项目的所有历史版本快照列表，按版本号倒序排列。",
)
async def list_project_versions(
    project_id: ProjectIdPath,
    service: ProjectService = Depends(resolve_project_service),
) -> ApiResponse[ProjectVersionListResponse]:
    """查询项目历史版本快照列表。"""
    items = service.list_project_versions(project_id)
    return ApiResponse(
        data=ProjectVersionListResponse(
            project_id=project_id,
            total=len(items),
            items=[to_project_version_item(item) for item in items],
        )
    )


@router.post(
    "/{project_id}/versions/{version_id}/rollback",
    response_model=ApiResponse[ProjectRollbackResponse],
    summary="回退项目至指定历史版本快照",
    description="一键将项目章节及内容回滚至指定历史版本快照（后悔药机制）。",
)
async def rollback_project_version(
    project_id: ProjectIdPath,
    version_id: VersionIdPath,
    service: ProjectService = Depends(resolve_project_service),
) -> ApiResponse[ProjectRollbackResponse]:
    """一键回滚项目至指定历史版本快照。"""
    info = service.rollback_project_version(project_id, version_id)
    return ApiResponse(
        data=to_project_rollback_response(info),
        message=f"项目已成功回滚至版本 V{info.restored_version_number}",
    )


