# 上传单个文本文件

上传单个文本文件到服务端，返回文件的元信息与下载地址。

| 属性 | 值 |
| --- | --- |
| **接口地址** | `POST /api/v1/text-files/upload` |
| **所属模块** | 文本文件管理（`app/modules/text_files/`） |
| **请求格式** | `multipart/form-data` |
| **响应格式** | `application/json` |
| **认证方式** | 无（当前版本未接入鉴权） |
| **幂等性** | 否，每次上传生成新的 `file_id` |

## 接口描述

将本地文本文件以表单方式提交到服务端。服务端会依次完成：

1. **文件名清洗** —— 剥离路径分隔符、过滤危险字符，避免路径穿越。
2. **扩展名校验** —— 仅接受文本类扩展名，不在白名单内直接返回 `415`，文件不会落盘。
3. **流式写入** —— 按 1 MB 分块读取并写入磁盘，实时累计字节数。
4. **大小校验** —— 超过大小上限（默认 50 MB）立即中断，
   删除半成品文件并返回 `413`。
5. **写入 CSV 索引与内容** —— 将小说/文本文件元信息记录到 `.leo-studio/novels.csv`，并将纯文本全文内容写入 `.leo-studio/novel_contents.csv`，供后续查询、下载与在线预览使用。

文件以 `file_id` 作为唯一标识落盘，磁盘实际存储名为 `{file_id}{扩展名}`，
因此同名文件多次上传不会互相覆盖。

> **注意**：本接口**只接受五种扩展名**：`.txt` `.md` `.doc` `.docx` `.epub`。
> 其中 `.doc` `.docx` `.epub` 为二进制文档格式，可上传与下载，
> 但不支持[内容预览接口](./content.md)。
> 图片、视频、压缩包等其它二进制类型请使用各自独立的功能模块接口。

## 请求参数

### Header

| 名称 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `Content-Type` | `string` | 是 | 固定为 `multipart/form-data; boundary=...` |
| `X-Request-ID` | `string` | 否 | 自定义请求追踪 ID，未传则由服务端生成 |

### Form Data

| 名称 | 类型 | 必填 | 默认值 | 说明 |
| --- | --- | --- | --- | --- |
| `file` | `binary` | **是** | — | 待上传的文本文件。字段名固定为 `file` |
| `description` | `string` | 否 | `null` | 文件描述说明，原样回显在响应中 |

> **注意**：`file` 字段名不可更改，改为其他名称将返回 `422`。

## 响应字段

### `data` 对象（`TextFileUploadResponse`）

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `file_id` | `string` | 文件唯一标识，32 位十六进制字符串 |
| `filename` | `string` | 清洗后的原始文件名 |
| `stored_name` | `string` | 磁盘实际存储文件名 `{file_id}{ext}` |
| `extension` | `string` | 文件扩展名（含点号），如 `.txt` |
| `size` | `integer` | 文件大小，单位字节 |
| `size_human` | `string` | 可读的文件大小，如 `1.25 KB` |
| `content_type` | `string` | MIME 类型，按扩展名推导（见文末注意事项） |
| `category` | `string` | 文本文件归类，见下方取值 |
| `uploaded_at` | `string` | 上传时间，ISO 8601 格式（UTC） |
| `download_url` | `string` | 下载地址，形如 `/api/v1/text-files/{file_id}/download` |
| `description` | `string \| null` | 请求中传入的文件描述 |

### `category` 取值

| 值 | 含义 | 典型扩展名 |
| --- | --- | --- |
| `plain_text` | 纯文本 | `.txt` |
| `markdown` | Markdown 文档 | `.md` |
| `word` | Word 文档 | `.doc` `.docx` |
| `epub` | 电子书 | `.epub` |
| `other` | 其他 | 通过白名单配置扩展时可能出现 |

## 成功示例

### 请求

```bash
curl -X POST "http://127.0.0.1:8000/api/v1/text-files/upload" \
  -F "file=@./notes.md" \
  -F "description=项目笔记"
```

```python
import httpx

with open("notes.md", "rb") as fp:
    response = httpx.post(
        "http://127.0.0.1:8000/api/v1/text-files/upload",
        files={"file": ("notes.md", fp, "text/markdown")},
        data={"description": "项目笔记"},
    )

result = response.json()
print(result["data"]["file_id"])
print(result["data"]["download_url"])
```

### 响应

`HTTP/1.1 200 OK`

```json
{
  "code": 0,
  "message": "文件上传成功",
  "data": {
    "file_id": "9f2c1a8b7d6e5f4a3b2c1d0e9f8a7b6c",
    "filename": "notes.md",
    "stored_name": "9f2c1a8b7d6e5f4a3b2c1d0e9f8a7b6c.md",
    "extension": ".md",
    "size": 2048,
    "size_human": "2.00 KB",
    "content_type": "text/markdown",
    "category": "markdown",
    "uploaded_at": "2026-10-22T14:32:18.451092+00:00",
    "download_url": "/api/v1/text-files/9f2c1a8b7d6e5f4a3b2c1d0e9f8a7b6c/download",
    "description": "项目笔记"
  },
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```

## 错误响应

### `415` 不支持的文件类型

文件扩展名不在文本文件白名单内，文件不会被保存。

```json
{
  "code": 41500,
  "message": "不支持的文件类型 .exe，允许的类型：.doc, .docx, .epub, .md, .txt",
  "data": null,
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```

### `413` 文件超过大小限制

默认上限为 50 MB（`LEO_MAX_UPLOAD_SIZE_MB`，
或模块级 `LEO_TEXT_FILES_MAX_UPLOAD_SIZE_MB`）。超限时服务端立即中断写入并删除半成品文件。

```json
{
  "code": 41300,
  "message": "文件大小超过限制，最大允许 50.00 MB",
  "data": null,
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```

### `422` 参数校验失败

常见原因：缺少 `file` 字段、文件名为空、文件内容为空（0 字节）。

```json
{
  "code": 40000,
  "message": "上传文件内容为空",
  "data": null,
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```

## 注意事项

- **文件名清洗**：上传名为 `../../etc/passwd.txt` 的文件时，响应中的 `filename`
  为 `passwd.txt`，实际落盘名为 `{file_id}.txt`，不会写入上传目录之外的任何位置。
- **空文件**：0 字节文件会被拒绝并返回 `422`。
- **同名文件**：每次上传都会生成新的 `file_id`，原始文件名相同不会覆盖已有文件。
- **content_type**：服务端不做 MIME 类型嗅探。扩展名是白名单的唯一准入依据，
  因此五种内置扩展名的 MIME 一律按扩展名推导
  （`.txt` → `text/plain`，`.md` → `text/markdown`，
  `.doc` → `application/msword`，
  `.docx` → `application/vnd.openxmlformats-officedocument.wordprocessingml.document`，
  `.epub` → `application/epub+zip`）；
  仅当通过 `LEO_TEXT_FILES_ALLOWED_EXTENSIONS` 追加了自定义扩展名时，
  才沿用客户端提交的值。
- **模块数据管理**：在 `.leo-studio` 目录下通过 `novels.csv`（元信息表）和 `novel_contents.csv`（小说全文内容表）持久化管理。

## 相关接口

- [下载文本文件](./download.md) —— 使用返回的 `file_id` 下载
- [查询文本文件列表](./list.md) —— 查看已上传的全部文本文件
