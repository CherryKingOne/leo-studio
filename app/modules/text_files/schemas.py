"""文本文件模块的请求与响应 Schema。

Schema 只描述接口契约，与领域模型解耦。
"""

from __future__ import annotations

from datetime import datetime

from pydantic import BaseModel, Field

from app.modules.text_files.models import TextFileCategory


class TextFileUploadResponse(BaseModel):
    """单文件上传结果。"""

    file_id: str = Field(description="文件唯一标识")
    filename: str = Field(description="清洗后的原始文件名")
    stored_name: str = Field(description="存储文件名，形如 {file_id}{ext}")
    extension: str = Field(description="文件扩展名，含点号")
    size: int = Field(description="文件大小，单位字节")
    size_human: str = Field(description="可读的文件大小")
    content_type: str = Field(description="MIME 类型")
    category: TextFileCategory = Field(description="文本文件归类")
    uploaded_at: datetime = Field(description="上传时间")
    download_url: str = Field(description="文件下载地址")
    description: str | None = Field(default=None, description="文件描述")


class TextFileListItem(BaseModel):
    """文件列表项。"""

    file_id: str = Field(description="文件唯一标识")
    filename: str = Field(description="清洗后的原始文件名")
    extension: str = Field(description="文件扩展名")
    size: int = Field(description="文件大小，单位字节")
    size_human: str = Field(description="可读的文件大小")
    category: TextFileCategory = Field(description="文本文件归类")
    uploaded_at: datetime = Field(description="上传时间")
    download_url: str = Field(description="文件下载地址")


class TextFileListResponse(BaseModel):
    """文件列表分页结果。"""

    total: int = Field(description="符合条件的文件总数")
    page: int = Field(description="当前页码，从 1 开始")
    page_size: int = Field(description="每页数量")
    pages: int = Field(description="总页数")
    items: list[TextFileListItem] = Field(description="文件列表")


class TextFileDeleteResponse(BaseModel):
    """删除文件结果。"""

    file_id: str = Field(description="文件唯一标识")
    filename: str = Field(description="被删除文件的原始文件名")
    deleted: bool = Field(description="是否删除成功")


class TextFileContentResponse(BaseModel):
    """文本文件内容响应。

    用于在线预览文本内容，区别于二进制下载接口。
    """

    file_id: str = Field(description="文件唯一标识")
    filename: str = Field(description="清洗后的原始文件名")
    content_type: str = Field(description="MIME 类型")
    encoding: str = Field(description="解码使用的字符集")
    size: int = Field(description="文件大小，单位字节")
    content: str = Field(description="文本内容")


class TextFileSentenceItem(BaseModel):
    """文本文件章节切句项。"""

    sentence_id: str = Field(description="句子唯一标识")
    novel_id: str = Field(description="小说 ID / 文件 ID")
    chapter_id: str = Field(description="所属章节 ID")
    sentence_index: int = Field(description="句子在章节中的行序号，从 0 开始")
    content: str = Field(description="句子正文内容")
    word_count: int = Field(description="句子字数")


class TextFileSentenceListResponse(BaseModel):
    """小说切句列表响应。"""

    novel_id: str = Field(description="小说 ID / 文件 ID")
    total: int = Field(description="切句总数")
    items: list[TextFileSentenceItem] = Field(description="切句列表")
