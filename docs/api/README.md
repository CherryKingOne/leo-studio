# API 接口文档

Leo Studio 后端服务的 HTTP 接口文档。所有接口基于 **FastAPI** 实现，
遵循 OpenAPI 3.1 规范，运行时可通过 `/docs`（Swagger UI）与 `/redoc` 查看交互式文档。

## 基本信息

| 项目 | 说明 |
| --- | --- |
| 服务名称 | Leo Studio API |
| 基础地址 | `http://127.0.0.1:8000` |
| 业务接口前缀 | `/api/v1` |
| 文本文件接口前缀 | `/api/v1/text-files` |
| 数据格式 | `application/json; charset=utf-8` |
| 文件上传格式 | `multipart/form-data` |
| 交互式文档 | `/docs`（Swagger UI）、`/redoc`（ReDoc） |
| OpenAPI 描述 | `/openapi.json` |

## 统一响应结构

所有业务接口（下载接口除外）均返回统一的 JSON 包装结构。

### 成功响应

```json
{
  "code": 0,
  "message": "success",
  "data": {},
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```

### 失败响应

```json
{
  "code": 41500,
  "message": "不支持的文件类型 .exe，允许的类型：.doc, .docx, .epub, .md, .txt",
  "data": null,
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `code` | `integer` | 业务状态码，`0` 表示成功，非 `0` 表示失败 |
| `message` | `string` | 人类可读的结果描述，失败时为具体原因 |
| `data` | `object \| null` | 业务数据，失败时通常为 `null` 或校验详情 |
| `request_id` | `string \| null` | 请求追踪 ID，同时写入响应头 `X-Request-ID`；客户端可通过请求头 `X-Request-ID` 传入自定义值 |

### 状态码对照表

| HTTP 状态码 | `code` | 含义 | 典型场景 |
| --- | --- | --- | --- |
| `200` | `0` | 请求成功 | 上传、查询、删除成功 |
| `404` | `40400` | 资源不存在 | 查询或删除不存在的 `file_id` |
| `409` | `40900` | 资源冲突 | 文件名冲突（预留） |
| `413` | `41300` | 请求体过大 | 上传文件超过大小上限 |
| `415` | `41500` | 媒体类型不支持 | 文件扩展名不在白名单内 |
| `422` | `40000` | 参数校验失败 | 文件名为空、文件内容为空、缺少必填字段 |
| `500` | `50000` | 存储异常 | 磁盘写入/读取失败 |
| `500` | `50001` | 服务器内部错误 | 未捕获的未预期异常 |

## 接口一览

接口按**业务功能**分组，每个功能模块拥有独立的路由前缀，
新增同类接口时请放入对应模块，避免路由冲突与职责混杂。

### 文本文件管理

模块目录：`app/modules/text_files/`　路由前缀：`/api/v1/text-files`

| 接口 | 方法 | 路径 | 说明 | 文档 |
| --- | --- | --- | --- | --- |
| 上传单个文本文件 | `POST` | `/api/v1/text-files/upload` | 上传单个文本文件并返回元信息 | [upload.md](./text-files/upload.md) |
| 查询文本文件列表 | `GET` | `/api/v1/text-files` | 分页查询已上传的文本文件 | [list.md](./text-files/list.md) |
| 查询文本文件详情 | `GET` | `/api/v1/text-files/{file_id}` | 获取单个文本文件的完整元信息 | [file.md](./text-files/file.md) |
| 预览文本文件内容 | `GET` | `/api/v1/text-files/{file_id}/content` | 以 JSON 返回解码后的文本内容 | [content.md](./text-files/content.md) |
| 查询小说章节切句 | `GET` | `/api/v1/text-files/{file_id}/sentences` | 获取按行切分后的小说句子列表 | - |
| 下载文本文件 | `GET` | `/api/v1/text-files/{file_id}/download` | 下载文件内容 | [download.md](./text-files/download.md) |
| 删除文本文件 | `DELETE` | `/api/v1/text-files/{file_id}` | 删除文件及其索引记录 | [delete.md](./text-files/delete.md) |

> 该模块**只接受 `.txt` `.md` `.doc` `.docx` `.epub` 五种扩展名**。
> 图片、视频、压缩包等其它二进制类型请新建独立功能模块
> （例如 `app/modules/images/`），不要混入文本文件模块。

### 剧目与项目管理

模块目录：`app/modules/projects/`　路由前缀：`/api/v1/projects`

| 接口 | 方法 | 路径 | 说明 | 文档 |
| --- | --- | --- | --- | --- |
| 创建项目并上传小说 | `POST` | `/api/v1/projects` | 上传小说文件并绑定项目名称，复用文本文件模块进行清洗、分章与切句 | [create.md](./projects/create.md) |
| 查询项目列表 | `GET` | `/api/v1/projects` | 分页查询活跃项目列表，按创建时间倒序（自动过滤回收站项目） | [list.md](./projects/list.md) |
| 查询回收站项目列表 | `GET` | `/api/v1/projects/recycle-bin` | 分页查询回收站中的项目列表，按删除时间倒序 | [recycle-bin.md](./projects/recycle-bin.md) |
| 清空回收站 | `DELETE` | `/api/v1/projects/recycle-bin` | 批量彻底删除回收站内的所有项目及其物理关联文件 | [recycle-bin.md](./projects/recycle-bin.md) |
| 查询项目详情 | `GET` | `/api/v1/projects/{project_id}` | 获取单个项目详细信息（回收站内项目返回 404） | [project.md](./projects/project.md) |
| 查询项目章节大纲列表 | `GET` | `/api/v1/projects/{project_id}/chapters` | 获取小说分章大纲列表（目录树），按序号排列 | [chapters.md](./projects/chapters.md) |
| 查询项目章节详细内容 | `GET` | `/api/v1/projects/{project_id}/chapters/{chapter_id}` | 获取单章清洗后完整正文与切句列表 | [chapter-content.md](./projects/chapter-content.md) |
| 追加上传项目章节文件 | `POST` | `/api/v1/projects/{project_id}/chapters/upload` | 增量追加小说章节文件，智能匹配防爬虫污染 | [append-chapters.md](./projects/append-chapters.md) |
| 软删除项目（移入回收站） | `DELETE` | `/api/v1/projects/{project_id}` | 将项目移入回收站，保留小说及切句数据支持随时恢复 | [delete.md](./projects/delete.md) |
| 从回收站恢复项目 | `POST` | `/api/v1/projects/{project_id}/restore` | 将回收站中的项目恢复为正常活跃状态 | [restore.md](./projects/restore.md) |
| 彻底删除项目（物理硬删除） | `DELETE` | `/api/v1/projects/{project_id}/hard` | 永久物理删除项目及其关联的小说文件、章节大纲与切句记录 | [hard-delete.md](./projects/hard-delete.md) |


### 根路径

| 接口 | 方法 | 路径 | 说明 | 文档 |
| --- | --- | --- | --- | --- |
| 服务信息 | `GET` | `/` | 返回服务名称、版本与模块索引 | [index.md](./routes/root/index.md) |
| 应用健康检查 | `GET` | `/health` | 返回服务健康状态 | [health.md](./routes/root/health.md) |

### 系统模块

模块目录：`app/modules/system/`　路由前缀：`/api/v1/system`

| 接口 | 方法 | 路径 | 说明 | 文档 |
| --- | --- | --- | --- | --- |
| 系统健康检查 | `GET` | `/api/v1/system/health` | 带版本前缀的健康检查 | [health.md](./routes/system/health.md) |

## 通用约定

### 文件标识符

- `file_id`：服务端生成的 32 位十六进制字符串（UUID4 去连字符），全局唯一。
- `stored_name`：磁盘实际存储文件名，格式为 `{file_id}{extension}`，
  例如 `9f2c1a8b7d6e5f4a3b2c1d0e9f8a7b6c.txt`。原始文件名保存在元数据索引中。
- `filename`：用户上传时的原始文件名（已做安全清洗，剥离路径分隔符）。
- `category`：文本文件归类，取值为 `plain_text`、`markdown`、`word`、
  `epub`、`other` 之一。

### 安全约束

| 约束 | 默认值 | 环境变量 |
| --- | --- | --- |
| 单文件大小上限 | `50 MB` | `LEO_MAX_UPLOAD_SIZE_MB`（全局）、`LEO_TEXT_FILES_MAX_UPLOAD_SIZE_MB`（模块） |
| 扩展名白名单 | 见下方 | `LEO_TEXT_FILES_ALLOWED_EXTENSIONS`（逗号分隔，留空表示不限制） |
| 文件名最大长度 | `255` 字符 | `LEO_TEXT_FILES_MAX_FILENAME_LENGTH` |
| 存储目录 | `storage/uploads/text-files` | `LEO_TEXT_FILES_UPLOAD_DIR` |

文本文件模块默认允许的扩展名：

| 分类 | 扩展名 |
| --- | --- |
| 纯文本 | `.txt` `.md` |
| Word 文档 | `.doc` `.docx` |
| 电子书 | `.epub` |

> **注意**：`.doc` `.docx` `.epub` 为二进制文档格式，不支持文本内容预览接口
> （`GET /api/v1/text-files/{file_id}/content`），请使用下载接口获取原始文件。

上传时服务端会：

1. **剥离路径**：取文件名中最后一个 `/` 或 `\` 之后的部分，阻断 `../` 路径穿越。
2. **过滤字符**：将非字母数字、下划线、中日韩文字、中划线、点的字符替换为 `_`。
3. **校验扩展名**：不在白名单内直接拒绝，不会落盘。
4. **流式写入**：按 1 MB 分块读写，实时累计字节数，超限立即中断并清理半成品文件。
5. **结构化存储**：纯文本小说会自动提取章节并按行切句，分别落盘至 `.leo-studio/novels.csv`、`novel_contents.csv` 与 `novel_sentences.csv`，各模块与表数据相互隔离。

### 错误排查

所有响应都带有 `X-Request-ID` 响应头，统一 JSON 包装结构的响应体中也包含
同值的 `request_id` 字段，可用于日志检索与问题定位。
若接口返回 `50001`，请携带该 `request_id` 反馈给服务端以定位异常堆栈。

## 相关文档

- [快速开始](./getting-started.md)
- [配置说明](./configuration.md)
- [错误码完整说明](./errors.md)
- [路由文档索引](./routes/)
