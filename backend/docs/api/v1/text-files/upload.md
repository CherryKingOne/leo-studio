# 上传文本文件

上传单个文本文件到服务端，服务端完成清洗、落盘、章节切分与切句，并返回文件元信息与下载地址。

| 属性 | 值 |
| --- | --- |
| **接口地址** | `POST /api/v1/text-files`（别名：`POST /api/v1/text-files/upload`） |
| **所属模块** | 文本文件（`src/modules/text_files/`） |
| **请求格式** | `multipart/form-data` |
| **响应格式** | `application/json` |
| **认证方式** | 无（当前版本未接入鉴权） |
| **幂等性** | 否，每次上传生成新的 `file_id` |

## 接口描述

将本地文本文件以表单方式提交到服务端，处理流程：

1. **文件名清洗** —— 剥离路径分隔符、过滤危险字符，避免路径穿越。
2. **扩展名校验** —— 默认仅允许 `.txt`，不在白名单内返回 `415`，文件不会落盘。
3. **大小校验** —— 默认上限 50 MB，超限返回 `413` 并中断写入。
4. **内容解码与清洗** —— 优先 UTF-8，失败时回退 GB18030；清洗颜文字与无意义占位行。
5. **章节切分与切句** —— 智能识别章节标题切分章节，章节内按行切句。
6. **落盘与持久化** —— 文件以 `{file_id}{扩展名}` 存入上传目录，元信息写入 `novels.csv`，
   章节写入 `novel_contents.csv`，切句写入 `novel_sentences.csv`。

## 请求参数

### Header

| 名称 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `Content-Type` | `string` | 是 | 固定为 `multipart/form-data; boundary=...` |
| `X-Request-ID` | `string` | 否 | 自定义请求追踪 ID，未传则由服务端生成 |

### Form Data

| 名称 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `file` | `binary` | **是** | 待上传文件，字段名固定为 `file` |
| `description` | `string` | 否 | 文件描述，原样回显在响应中 |

## 响应字段

### `data` 对象（`TextFileUploadResponse`）

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `file_id` | `string` | 文件唯一标识，32 位十六进制字符串 |
| `filename` | `string` | 清洗后的原始文件名 |
| `stored_name` | `string` | 磁盘实际存储文件名 `{file_id}{ext}` |
| `extension` | `string` | 扩展名（含点号），如 `.txt` |
| `size` | `integer` | 文件大小，单位字节 |
| `size_human` | `string` | 可读大小，如 `1.25 KB` |
| `content_type` | `string` | MIME 类型，`.txt` 固定为 `text/plain` |
| `category` | `string` | 归类：`.txt` 为 `plain_text`，其它为 `other` |
| `uploaded_at` | `string` | 上传时间，RFC 3339（UTC） |
| `download_url` | `string` | 下载地址，`/api/v1/text-files/{file_id}/download` |
| `description` | `string \| null` | 请求传入的描述（未传时省略该字段） |

## 代码示例

### curl

```bash
curl -X POST "http://127.0.0.1:58685/api/v1/text-files" \
  -F "file=@./example.txt" \
  -F "description=示例文本"
```

## 成功响应示例

`HTTP/1.1 200 OK`

```json
{
  "code": 0,
  "message": "文件上传成功",
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
    "download_url": "/api/v1/text-files/9f2c1a8b7d6e5f4a3b2c1d0e9f8a7b6c/download",
    "description": "示例文本"
  },
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```

## 失败响应示例

### 415 不支持的文件类型

```json
{
  "code": 41500,
  "message": "不支持的文件类型 .exe，允许的类型：.txt",
  "data": null,
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```

### 413 文件超过大小限制

```json
{
  "code": 41300,
  "message": "文件大小超过限制，最大允许 50.00 MB",
  "data": null,
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```

### 422 参数校验失败

```json
{
  "code": 40000,
  "message": "上传文件内容为空",
  "data": null,
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```

## 注意事项

- 文件名清洗后如为空或超长（默认 255 字符）会返回 `422`。
- 同名文件多次上传不会覆盖，每次都生成新的 `file_id`。
- 白名单与大小上限可通过环境变量 `LEO_TEXT_FILES_ALLOWED_EXTENSIONS`、
  `LEO_MAX_UPLOAD_SIZE_MB`、`LEO_TEXT_FILES_MAX_UPLOAD_SIZE_MB` 调整。