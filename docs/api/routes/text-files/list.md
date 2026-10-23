# 查询文本文件列表

分页查询已上传的文本文件列表，按上传时间倒序排列。

| 属性 | 值 |
| --- | --- |
| **接口地址** | `GET /api/v1/text-files` |
| **所属模块** | 文本文件管理（`app/modules/text_files/`） |
| **请求格式** | 无请求体 |
| **响应格式** | `application/json` |
| **认证方式** | 无（当前版本未接入鉴权） |
| **幂等性** | 是（只读操作） |

## 接口描述

返回服务端已保存的文本文件元信息列表，支持分页。排序规则为**上传时间倒序**
（最新上传的文件排在最前）。

该接口只返回元信息，不包含文件内容。获取文件内容请使用
[下载接口](./download.md)或[内容预览接口](./content.md)。

## 请求参数

### Query

| 名称 | 类型 | 必填 | 默认值 | 约束 | 说明 |
| --- | --- | --- | --- | --- | --- |
| `page` | `integer` | 否 | `1` | `>= 1` | 页码，从 1 开始 |
| `page_size` | `integer` | 否 | `20` | `1 <= n <= 100` | 每页数量，最大 100 |

### Header

| 名称 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `X-Request-ID` | `string` | 否 | 自定义请求追踪 ID，未传则由服务端生成 |

## 响应字段

### `data` 对象（`TextFileListResponse`）

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `total` | `integer` | 符合条件的文件总数 |
| `page` | `integer` | 当前页码 |
| `page_size` | `integer` | 每页数量 |
| `pages` | `integer` | 总页数，`total` 为 0 时为 `0` |
| `items` | `TextFileListItem[]` | 当前页的文件列表 |

### `items[]` 元素（`TextFileListItem`）

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `file_id` | `string` | 文件唯一标识 |
| `filename` | `string` | 清洗后的原始文件名 |
| `extension` | `string` | 文件扩展名（含点号） |
| `size` | `integer` | 文件大小，单位字节 |
| `size_human` | `string` | 可读的文件大小 |
| `category` | `string` | 文本文件归类（`plain_text`/`markdown`/`word`/`epub`/`other`） |
| `uploaded_at` | `string` | 上传时间，ISO 8601 格式（UTC） |
| `download_url` | `string` | 下载地址 |

> 与[上传接口](./upload.md)的响应相比，列表项不包含 `stored_name`、
> `content_type` 与 `description` 字段，以减少响应体积。

## 成功示例

### 请求

```bash
curl -X GET "http://127.0.0.1:8000/api/v1/text-files?page=1&page_size=20"
```

```python
import httpx

response = httpx.get(
    "http://127.0.0.1:8000/api/v1/text-files",
    params={"page": 1, "page_size": 20},
)

data = response.json()["data"]
print(f"共 {data['total']} 个文件，第 {data['page']}/{data['pages']} 页")
for item in data["items"]:
    print(item["file_id"], item["filename"], item["size_human"])
```

### 响应

`HTTP/1.1 200 OK`

```json
{
  "code": 0,
  "message": "success",
  "data": {
    "total": 42,
    "page": 1,
    "page_size": 20,
    "pages": 3,
    "items": [
      {
        "file_id": "9f2c1a8b7d6e5f4a3b2c1d0e9f8a7b6c",
        "filename": "notes.md",
        "extension": ".md",
        "size": 2048,
        "size_human": "2.00 KB",
        "category": "markdown",
        "uploaded_at": "2026-10-22T14:32:18.451092+00:00",
        "download_url": "/api/v1/text-files/9f2c1a8b7d6e5f4a3b2c1d0e9f8a7b6c/download"
      },
      {
        "file_id": "1a2b3c4d5e6f70819a2b3c4d5e6f7081",
        "filename": "report.docx",
        "extension": ".docx",
        "size": 524288,
        "size_human": "512.00 KB",
        "category": "word",
        "uploaded_at": "2026-10-22T14:30:02.118753+00:00",
        "download_url": "/api/v1/text-files/1a2b3c4d5e6f70819a2b3c4d5e6f7081/download"
      }
    ]
  },
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```

## 错误响应

### `422` 参数校验失败

`page` 小于 1 或 `page_size` 超出 `1..100` 范围时返回。

```json
{
  "code": 40000,
  "message": "请求参数校验失败",
  "data": [
    {
      "type": "greater_than_equal",
      "loc": ["query", "page_size"],
      "msg": "Input should be greater than or equal to 1",
      "input": "0"
    }
  ],
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```

## 注意事项

- **空列表**：尚未上传任何文件时，`total` 与 `pages` 均为 `0`，`items` 为空数组。
- **越界页码**：请求的页码超出总页数时，`items` 返回空数组，不报错。
- **模块范围**：只返回文本文件模块内上传的文件，不包含其它功能模块的数据。
- **索引存储**：当前版本使用 JSON 文件（`.text-files-index.json`）存储元信息，
  单机适用；如需多实例部署应替换为数据库实现。

## 相关接口

- [上传单个文本文件](./upload.md) —— 上传文件
- [查询文本文件详情](./file.md) —— 获取单个文件的完整元信息
- [删除文本文件](./delete.md) —— 删除文件
