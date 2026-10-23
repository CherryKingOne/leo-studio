"""文本文件模块的常量与类型定义。

模块内所有硬编码的常量集中在此，便于统一维护与查阅。

==============================================================================
【后续如何新增受支持的文件格式 - 完整开发流程指引】
==============================================================================
当需要为系统接入新的文件格式（如 .md, .epub, .doc/.docx, .pdf 等）时，
请按以下步骤依次修改相关代码：

步骤 1：常量配置（当前文件 constants.py）
  1.1 在下方 `TEXT_FILE_EXTENSIONS` 白名单集合中添加新扩展名（如 `".md"`）。
  1.2 在下方 `EXTENSION_MIME_MAP` 映射字典中配置默认 MIME 类型（如 `".md": "text/markdown"`）。
  1.3 若新增的格式属于二进制文档（无法直接以 UTF-8/GB18030 等纯文本解码预览）：
      - 建议定义 `BINARY_EXTENSIONS` 集合记录此类扩展名。

步骤 2：分类定义与映射（models.py）
  2.1 若新格式需要专属类别，在 `TextFileCategory` 枚举中新增枚举项（如 `MARKDOWN = "markdown"`）。
  2.2 在 `EXTENSION_CATEGORY_MAP` 中将扩展名映射到该类别（如 `".md": TextFileCategory.MARKDOWN`）。

步骤 3：正文解析与预览处理（service.py）
  3.1 检查 `is_previewable(extension)`：
      - 纯文本类格式返回 True（支持 /content 接口在线预览）。
      - 二进制格式需返回 False，避免直接当文本解码引发异常。
  3.2 检查 `save_file(...)` 中的正文与章节提取：
      - 纯文本格式直接调用 `read_text` 读取并使用 `parse_chapters` 切分章节。
      - 二进制/富文本格式（如 EPUB、DOCX、PDF），若需要提取章节正文入库，
        需编写专属解析器（提取文本后再切分章节）或仅作二进制存储。
  3.3 检查 `read_text(...)`：
      - 二进制格式应拦截并抛出 `InvalidParameterError`，引导用户使用下载接口。

步骤 4：路由说明文档（router.py）
  4.1 更新 `/upload` 接口的 docstring，在支持的扩展名列表中补充新格式。
  4.2 若新增格式不支持预览，更新 `/{file_id}/content` 接口的 docstring 说明。

步骤 5：测试与配置更新（tests/）
  5.1 检查 `tests/conftest.py` 中的 `test_settings`，若显式配置了 `allowed_extensions`，
      需同步添加新扩展名。
  5.2 在 `tests/test_text_files_api.py` 中为新扩展名补充单测（包括上传、下载、MIME 推导、
      预览是否正常或是否被正确拦截等）。
==============================================================================
"""

from __future__ import annotations

from typing import Final

#: 模块路由前缀，与 api_router 的 /api/v1 组合后为 /api/v1/text-files
ROUTE_PREFIX: Final[str] = "/text-files"

#: 模块在 OpenAPI 中的标签名
MODULE_TAG: Final[str] = "文本文件"

#: 元数据索引文件名，与其它功能模块隔离，避免互相覆盖
INDEX_FILENAME: Final[str] = ".text-files-index.json"

#: 小说元数据与内容存储目录（相对于项目根目录）
DEFAULT_CSV_DIR: Final[str] = ".leo-studio"

#: 小说元信息表文件名
NOVELS_CSV_FILENAME: Final[str] = "novels.csv"

#: 小说内容表文件名
NOVEL_CONTENTS_CSV_FILENAME: Final[str] = "novel_contents.csv"

#: 小说章节切句表文件名
NOVEL_SENTENCES_CSV_FILENAME: Final[str] = "novel_sentences.csv"

#: 小说元信息表表头字段（纯小说元信息，包含格式 format，不含 author、genre、avg_words_per_chapter 及底层存储技术字段）
NOVELS_COLUMNS: Final[list[str]] = [
    "novel_id",
    "title",
    "format",
    "chapter_count",
    "total_words",
    "line_count",
    "created_at",
]

#: 小说内容表表头字段（按章节管理，包含章节名称与正文，通过 novel_id 关联）
NOVEL_CONTENTS_COLUMNS: Final[list[str]] = [
    "chapter_id",
    "novel_id",
    "chapter_index",
    "chapter_title",
    "content",
    "word_count",
]

#: 小说章节切句表表头字段（按行切句，通过 novel_id 与 chapter_id 关联）
NOVEL_SENTENCES_COLUMNS: Final[list[str]] = [
    "sentence_id",
    "novel_id",
    "chapter_id",
    "sentence_index",
    "content",
    "word_count",
]

#: 文本文件内置白名单，可通过环境变量 LEO_TEXT_FILES_ALLOWED_EXTENSIONS 覆盖。
#: 【新增格式】：在此集合中追加新扩展名，例如 `".md"`、`".epub"` 等（注意带点前缀）。
TEXT_FILE_EXTENSIONS: Final[frozenset[str]] = frozenset(
    {
        # 纯文本
        ".txt",
    }
)

#: 扩展名到默认 MIME 类型的映射。
#: 客户端未提交 content_type 时使用该表兜底。
#: 【新增格式】：在此字典中补充新扩展名对应的标准 MIME 类型（如 `".md": "text/markdown"`）。
EXTENSION_MIME_MAP: Final[dict[str, str]] = {
    ".txt": "text/plain",
}
