"""项目管理模块的请求与响应 Schema。"""

from __future__ import annotations

from datetime import datetime

from pydantic import BaseModel, Field


class ProjectResponse(BaseModel):
    """单个项目详情响应。"""

    project_id: str = Field(description="项目唯一标识 (UUID)")
    name: str = Field(description="项目名称")
    file_id: str = Field(description="关联文本文件 ID (UUID)")
    filename: str = Field(description="原始文件名")
    status: str = Field(default="制作中", description="制作状态")
    chapter_count: int = Field(default=0, description="章节数")
    total_words: int = Field(default=0, description="总字数")
    created_at: datetime = Field(description="创建时间")
    updated_at: datetime = Field(description="最后更新时间")


class ProjectListItem(BaseModel):
    """项目列表项。"""

    project_id: str = Field(description="项目唯一标识 (UUID)")
    name: str = Field(description="项目名称")
    file_id: str = Field(description="关联文本文件 ID (UUID)")
    filename: str = Field(description="原始文件名")
    status: str = Field(default="制作中", description="制作状态")
    chapter_count: int = Field(default=0, description="章节数")
    total_words: int = Field(default=0, description="总字数")
    created_at: datetime = Field(description="创建时间")
    updated_at: datetime = Field(description="最后更新时间")


class ProjectListResponse(BaseModel):
    """项目列表分页响应。"""

    total: int = Field(description="总记录数")
    page: int = Field(description="当前页码")
    page_size: int = Field(description="每页记录数")
    pages: int = Field(description="总页数")
    items: list[ProjectListItem] = Field(default_factory=list, description="项目列表")


class ProjectDeleteResponse(BaseModel):
    """项目删除响应。"""

    project_id: str = Field(description="被删除的项目 ID (UUID)")
    name: str = Field(description="项目名称")
    deleted: bool = Field(default=True, description="是否删除成功")


class ChapterOutlineItem(BaseModel):
    """章节大纲列表项。"""

    chapter_id: str = Field(description="章节唯一标识 (UUID)")
    chapter_index: int = Field(description="章节序号（从 0 开始）")
    chapter_title: str = Field(description="章节标题")
    word_count: int = Field(default=0, description="章节字数")


class ChapterOutlineListResponse(BaseModel):
    """项目章节大纲列表响应。"""

    project_id: str = Field(description="项目唯一标识 (UUID)")
    total: int = Field(description="章节总数")
    items: list[ChapterOutlineItem] = Field(
        default_factory=list, description="章节大纲列表"
    )


class ChapterSentenceItem(BaseModel):
    """章节切句项。"""

    sentence_id: str = Field(description="句子唯一标识 (UUID)")
    sentence_index: int = Field(description="句子在章节内的行索引（从 0 开始）")
    content: str = Field(description="句子正文内容")
    word_count: int = Field(default=0, description="句子字数")


class ChapterContentResponse(BaseModel):
    """章节详细内容与切句响应。"""

    project_id: str = Field(description="项目唯一标识 (UUID)")
    file_id: str = Field(description="关联文本文件 ID (UUID)")
    chapter_id: str = Field(description="章节唯一标识 (UUID)")
    chapter_index: int = Field(description="章节序号（从 0 开始）")
    chapter_title: str = Field(description="章节标题")
    word_count: int = Field(default=0, description="章节字数")
    content: str = Field(description="章节完整正文文本")
    sentences: list[ChapterSentenceItem] = Field(
        default_factory=list, description="章节行级切句列表"
    )


class ChapterConflictItem(BaseModel):
    """章节内容冲突差异项。"""

    chapter_id: str = Field(description="冲突章节 ID (UUID)")
    chapter_index: int = Field(description="章节序号")
    chapter_title: str = Field(description="章节标题")
    existing_word_count: int = Field(description="现有章节字数")
    incoming_word_count: int = Field(description="新上传章节字数")
    word_diff: int = Field(description="字数差异（新 - 旧）")


class ProjectChapterAppendResponse(BaseModel):
    """项目追加章节响应。"""

    project_id: str = Field(description="项目唯一标识 (UUID)")
    file_id: str = Field(description="关联文本文件 ID (UUID)")
    added_chapters: int = Field(description="本次新增章节数量")
    skipped_chapters: int = Field(description="本次跳过（或保留）的已存在章节数量")
    total_chapters: int = Field(description="项目最新总章节数")
    total_words: int = Field(description="项目最新总字数")
    has_conflicts: bool = Field(default=False, description="是否存在内容差异冲突")
    conflicts: list[ChapterConflictItem] = Field(
        default_factory=list, description="冲突章节差异列表"
    )
    updated_at: datetime = Field(description="最后更新时间")



