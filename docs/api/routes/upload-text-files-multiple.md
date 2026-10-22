# 批量上传文本文件

一次提交多个文本文件，全部成功或整体回滚。

| 属性 | 值 |
| --- | --- |
| **接口地址** | `POST /api/v1/text-files/upload/multiple` |
| **所属模块** | 文本文件管理（`app/modules/text_files/`） |
| **请求格式** | `multipart/form-data` |
| **响应格式** | `application/json` |
| **认证方式** | 无（当前版本未接入鉴权） |
| **幂等性** | 否，每次上传生成新的 `file_id` |

## 接口描述

与[单文件上传](./upload-text-file.md)共用同一套校验与落盘逻辑，区别在于：

- 表单字段名为 `files`，可**重复出现**多次。
- 响应返回一个数组，包含每个文件的元信息。

> **注意**：本接口采用**全成功或全回滚**策略。若列表中任意一个文件校验失败
> （类型不支持、超限、空文件等），整个请求返回错误，
> 且本次调用已落盘的前序文件会被自动删除，索引记录一并清理，不留半成品数据。

## 请求参数

### Header

| 名称 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `Content-Type` | `string` | 是 | 固定为 `multipart/form-data; boundary=...` |

### Form Data

| 名称 | 类型 | 必填 | 默认值 | 说明 |
| --- | --- | --- | --- | --- |
| `files` | `binary[]` | **是** | — | 待上传的文本文件列表。字段名固定为 `files`，可重复 |

## 响应字段

### `data` 对象（`MultiTextFileUploadResponse`）

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `total` | `integer` | 本次成功上传的文件数量 |
| `files` | `TextFileUploadResponse[]` | 每个文件的元信息列表，字段含义见下 |

### `files[]` 元素（`TextFileUploadResponse`）

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `file_id` | `string` | 文件唯一标识 |
| `filename` | `string` | 清洗后的原始文件名 |
| `stored_name` | `string` | 磁盘实际存储文件名 |
| `extension` | `string` | 文件扩展名（含点号） |
| `size` | `integer` | 文件大小，单位字节 |
| `size_human` | `string` | 可读的文件大小 |
| `content_type` | `string` | MIME 类型 |
| `category` | `string` | 文本文件归类（`plain_text`/`markdown`/`data`/`config`/`source_code`/`log`/`other`） |
| `uploaded_at` | `string` | 上传时间，ISO 8601 格式（UTC） |
| `download_url` | `string` | 下载地址 |
| `description` | `string \| null` | 文件描述（本接口不传该字段，恒为 `null`） |

## 成功示例

### 请求

```bash
curl -X POST "http://127.0.0.1:8000/api/v1/text-files/upload/multiple" \
  -F "files=@./a.txt" \
  -F "files=@./b.md" \
  -F "files=@./c.json"
```

```python
import httpx

files = [
    ("files", ("a.txt", open("a.txt", "rb"), "text/plain")),
    ("files", ("b.md", open("b.md", "rb"), "text/markdown")),
    ("files", ("c.json", open("c.json", "rb"), "application/json")),
]

response = httpx.post(
    "http://127.0.0.1:8000/api/v1/text-files/upload/multiple", files=files
)

data = response.json()["data"]
print(data["total"])              # 3
for item in data["files"]:
    print(item["file_id"], item["filename"], item["size_human"])
```

### 响应

`HTTP/1.1 200 OK`

```json
{
  "code": 0,
  "message": "成功上传 3 个文件",
  "data": {
    "total": 3,
    "files": [
      {
        "file_id": "9f2c1a8b7d6e5f4a3b2c1d0e9f8a7b6c",
        "filename": "a.txt",
        "stored_name": "9f2c1a8b7d6e5f4a3b2c1d0e9f8a7b6c.txt",
        "extension": ".txt",
        "size": 1024,
        "size_human": "1.00 KB",
        "content_type": "text/plain",
        "category": "plain_text",
        "uploaded_at": "2026-10-22T14:32:18.451092+00:00",
        "download_url": "/api/v1/text-files/9f2c1a8b7d6e5f4a3b2c1d0e9f8a7b6c/download",
        "description": null
      },
      {
        "file_id": "1a2b3c4d5e6f70819a2b3c4d5e6f7081",
        "filename": "b.md",
        "stored_name": "1a2b3c4d5e6f70819a2b3c4d5e6f7081.md",
        "extension": ".md",
        "size": 2048,
        "size_human": "2.00 KB",
        "content_type": "text/markdown",
        "category": "markdown",
        "uploaded_at": "2026-10-22T14:32:18.463201+00:00",
        "download_url": "/api/v1/text-files/1a2b3c4d5e6f70819a2b3c4d5e6f7081/download",
        "description": null
      },
      {
        "file_id": "2b3c4d5e6f708192a3b4c5d6e7f80912",
        "filename": "c.json",
        "stored_name": "2b3c4d5e6f708192a3b4c5d6e7f80912.json",
        "extension": ".json",
        "size": 512,
        "size_human": "512 B",
        "content_type": "application/json",
        "category": "data",
        "uploaded_at": "2026-10-22T14:32:18.478913+00:00",
        "download_url": "/api/v1/text-files/2b3c4d5e6f708192a3b4c5d6e7f80912/download",
        "description": null
      }
    ]
  },
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```

## 错误响应

错误码与[单文件上传](./upload-text-file.md#错误响应)完全一致，此处不再重复列举。

| HTTP 状态码 | `code` | 典型场景 |
| --- | --- | --- |
| `415` | `41500` | 列表中某个文件扩展名不在文本文件白名单内 |
| `413` | `41300` | 列表中某个文件超过大小限制 |
| `422` | `40000` | 未提交任何文件，或某个文件内容为空 |

### `422` 未提交文件

```json
{
  "code": 40000,
  "message": "至少需要上传一个文件",
  "data": null,
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```

## 注意事项

- **自动回滚**：任意文件校验失败即返回错误，本次调用已保存的文件会被自动删除，
  索引记录同步清理，服务端不会残留半成品数据。
- **数量建议**：单次请求的文件数量不宜过多（如超过 20 个），
  以免单个 HTTP 请求超时。
- **类型一致**：所有文件都必须属于文本类扩展名，二进制类型请使用对应的功能模块接口。

## 相关接口

- [上传单个文本文件](./upload-text-file.md) —— 上传单个文本文件
- [查询文本文件列表](./list-text-files.md) —— 查看已上传的全部文本文件
