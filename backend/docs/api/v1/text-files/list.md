# 查询文本文件列表

分页查询已上传的全部文本文件，按上传时间倒序返回。

| 属性 | 值 |
| --- | --- |
| **接口地址** | `GET /api/v1/text-files`（别名：`GET /api/v1/text-files/upload`） |
| **所属模块** | 文本文件（`src/modules/text_files/`） |
| **请求格式** | 无请求体 |
| **响应格式** | `application/json` |
| **认证方式** | 无 |
| **幂等性** | 是 |

## 接口描述

返回 `novels.csv` 中记录的所有文本文件元信息，按 `uploaded_at` 倒序排列并分页。
当上传目录中存在对应文件时，`size` 会取磁盘真实大小；否则回退为记录中的字数。

## 请求参数

### Query

| 名称 | 类型 | 必填 | 默认值 | 说明 |
| --- | --- | --- | --- | --- |
| `page` | `integer` | 否 | `1` | 页码，必须大于 0 |
| `page_size` | `integer` | 否 | `20` | 每页数量，范围 1~100 |

### Header

| 名称 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `X-Request-ID` | `string` | 否 | 自定义请求追踪 ID |

## 响应字段

### `data` 对象（`TextFileListResponse`）

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `total` | `integer` | 总记录数 |
| `page` | `integer` | 当前页码 |
| `page_size` | `integer` | 每页数量 |
| `pages` | `integer` | 总页数（无数据为 0） |
| `items` | `object[]` | 文件列表，元素为 `TextFileListItem` |

### `items[]` 元素（`TextFileListItem`）

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `file_id` | `string` | 文件唯一标识 |
| `filename` | `string` | 清洗后的文件名 |
| `extension` | `string` | 扩展名（含点号） |
| `size` | `integer` | 文件大小（字节） |
| `size_human` | `string` | 可读大小 |
| `category` | `string` | 归类 |
| `uploaded_at` | `string` | 上传时间（RFC 3339 UTC） |
| `download_url` | `string` | 下载地址 |

## 代码示例

```bash
curl -s "http://127.0.0.1:58685/api/v1/text-files?page=1&page_size=20"
```

## 成功响应示例

```json
{
  "code": 0,
  "message": "success",
  "data": {
    "total": 1,
    "page": 1,
    "page_size": 20,
    "pages": 1,
    "items": [
      {
        "file_id": "9f2c1a8b7d6e5f4a3b2c1d0e9f8a7b6c",
        "filename": "example.txt",
        "extension": ".txt",
        "size": 2048,
        "size_human": "2.00 KB",
        "category": "plain_text",
        "uploaded_at": "2026-10-22T14:32:18.451092+00:00",
        "download_url": "/api/v1/text-files/9f2c1a8b7d6e5f4a3b2c1d0e9f8a7b6c/download"
      }
    ]
  },
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```

## 失败响应示例

### 422 分页参数非法

```json
{
  "code": 40000,
  "message": "每页数量必须在 1 到 100 之间",
  "data": null,
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```