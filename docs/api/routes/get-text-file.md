# 查询文本文件详情

根据 `file_id` 查询单个文本文件的完整元信息。

| 属性 | 值 |
| --- | --- |
| **接口地址** | `GET /api/v1/text-files/{file_id}` |
| **所属模块** | 文本文件管理（`app/modules/text_files/`） |
| **请求格式** | 无请求体 |
| **响应格式** | `application/json` |
| **认证方式** | 无（当前版本未接入鉴权） |
| **幂等性** | 是（只读操作） |

## 接口描述

返回指定文本文件的完整元信息。与[文本文件列表接口](./list-text-files.md)相比，
本接口额外返回 `stored_name`、`content_type` 与 `description` 字段。

接口只返回元信息，不包含文件内容。获取文件内容请使用
[下载接口](./download-text-file.md)或[内容预览接口](./get-text-file-content.md)。

## 路径参数

| 名称 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `file_id` | `string` | **是** | 文件唯一标识，32 位十六进制字符串 |

## 响应字段

### `data` 对象（`TextFileUploadResponse`）

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `file_id` | `string` | 文件唯一标识 |
| `filename` | `string` | 清洗后的原始文件名 |
| `stored_name` | `string` | 磁盘实际存储文件名 `{file_id}{ext}` |
| `extension` | `string` | 文件扩展名（含点号） |
| `size` | `integer` | 文件大小，单位字节 |
| `size_human` | `string` | 可读的文件大小 |
| `content_type` | `string` | MIME 类型，按扩展名推导 |
| `category` | `string` | 文本文件归类（`plain_text`/`markdown`/`word`/`epub`/`other`） |
| `uploaded_at` | `string` | 上传时间，ISO 8601 格式（UTC） |
| `download_url` | `string` | 下载地址 |
| `description` | `string \| null` | 上传时传入的文件描述，未传则为 `null` |

## 成功示例

### 请求

```bash
curl -X GET "http://127.0.0.1:8000/api/v1/text-files/9f2c1a8b7d6e5f4a3b2c1d0e9f8a7b6c"
```

```python
import httpx

file_id = "9f2c1a8b7d6e5f4a3b2c1d0e9f8a7b6c"
response = httpx.get(f"http://127.0.0.1:8000/api/v1/text-files/{file_id}")

data = response.json()["data"]
print(data["filename"], data["size_human"], data["category"])
```

### 响应

`HTTP/1.1 200 OK`

```json
{
  "code": 0,
  "message": "success",
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

### `404` 文件不存在

`file_id` 在索引中不存在，或对应的磁盘文件已被外部删除。

```json
{
  "code": 40400,
  "message": "文件不存在：does-not-exist",
  "data": null,
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```

## 注意事项

- **file_id 格式**：`file_id` 由服务端生成的 UUID4 去连字符组成，
  不做格式校验，任意字符串都会走查索引流程并返回 `404`。
- **索引一致性**：元信息来自 `.text-files-index.json` 索引文件。
  若磁盘文件被手工删除，详情接口仍会返回元信息，
  但[下载接口](./download-text-file.md)会返回 `404`。
- **模块范围**：只查询文本文件模块内的文件，其它模块的 `file_id` 同样返回 `404`。

## 相关接口

- [文本文件列表](./list-text-files.md) —— 分页查询全部文本文件
- [下载文本文件](./download-text-file.md) —— 下载文件内容
- [预览文本文件内容](./get-text-file-content.md) —— 以 JSON 返回文本内容
- [删除文本文件](./delete-text-file.md) —— 删除文件
