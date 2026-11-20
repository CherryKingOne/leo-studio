# 查询文本文件详情

根据 `file_id` 查询单个文本文件的完整元信息。

| 属性 | 值 |
| --- | --- |
| **接口地址** | `GET /api/v1/text-files/{file_id}` |
| **所属模块** | 文本文件（`src/modules/text_files/`） |
| **请求格式** | 无请求体 |
| **响应格式** | `application/json` |
| **认证方式** | 无 |
| **幂等性** | 是 |

## 接口描述

返回与上传接口一致的文件元信息结构。若上传目录中存在文件，`size` 取磁盘真实大小。

## 请求参数

### Path

| 名称 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `file_id` | `string` | **是** | 文件唯一标识 |

### Header

| 名称 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `X-Request-ID` | `string` | 否 | 自定义请求追踪 ID |

## 响应字段

### `data` 对象（`TextFileUploadResponse`）

字段同[上传文本文件](./upload.md#响应字段)。

## 代码示例

```bash
curl -s "http://127.0.0.1:58685/api/v1/text-files/9f2c1a8b7d6e5f4a3b2c1d0e9f8a7b6c"
```

## 成功响应示例

```json
{
  "code": 0,
  "message": "success",
  "data": {
    "file_id": "9f2c1a8b7d6e5f4a3b2c1d0e9f8a7b6c",
    "filename": "example.txt",
    "stored_name": "9f2c1a8b7d6e5f4a3b2c1d0e9f8a7b6c.txt",
    "extension": ".txt",
    "size": 2048,
    "size_human": "2.00 KB",
    "content_type": "text/plain",
    "category": "plain_text",
    "uploaded_at": "2026-10-22T14:32:18.451092+00:00",
    "download_url": "/api/v1/text-files/9f2c1a8b7d6e5f4a3b2c1d0e9f8a7b6c/download"
  },
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```

## 失败响应示例

### 404 文件不存在

```json
{
  "code": 40400,
  "message": "文件不存在：9f2c1a8b7d6e5f4a3b2c1d0e9f8a7b6c",
  "data": null,
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```