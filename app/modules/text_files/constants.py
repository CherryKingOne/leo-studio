"""文本文件模块的常量与类型定义。

模块内所有硬编码的常量集中在此，便于统一维护与查阅。
"""

from __future__ import annotations

from typing import Final

#: 模块路由前缀，与 api_router 的 /api/v1 组合后为 /api/v1/text-files
ROUTE_PREFIX: Final[str] = "/text-files"

#: 模块在 OpenAPI 中的标签名
MODULE_TAG: Final[str] = "文本文件"

#: 元数据索引文件名，与其它功能模块隔离，避免互相覆盖
INDEX_FILENAME: Final[str] = ".text-files-index.json"

#: 文本文件内置白名单，可通过 LEO_TEXT_FILES_ALLOWED_EXTENSIONS 覆盖
TEXT_FILE_EXTENSIONS: Final[frozenset[str]] = frozenset(
    {
        # 纯文本与 Markdown
        ".txt",
        ".md",
        # Word 文档
        ".doc",
        ".docx",
        # 电子书
        ".epub",
    }
)

#: 扩展名到默认 MIME 类型的映射。
#: 客户端未提交 content_type 时使用该表兜底，避免二进制格式被标成 text/plain。
EXTENSION_MIME_MAP: Final[dict[str, str]] = {
    ".txt": "text/plain",
    ".md": "text/markdown",
    ".doc": "application/msword",
    ".docx": "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
    ".epub": "application/epub+zip",
}

#: 二进制文档扩展名，这些文件不是可直接解码的纯文本，不支持内容预览接口。
BINARY_EXTENSIONS: Final[frozenset[str]] = frozenset({".doc", ".docx", ".epub"})
