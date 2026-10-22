# 错误码说明

所有业务接口失败时返回统一的 JSON 包装结构：

```json
{
  "code": 41500,
  "message": "不支持的文件类型 .exe，允许的类型：...",
  "data": null,
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```

`code` 为 `0` 表示成功，非 `0` 表示业务或系统错误。

## 错误码总表

| HTTP 状态码 | `code` | 常量 | 含义 | 处理建议 |
| --- | --- | --- | --- | --- |
| `200` | `0` | `ErrorCode.SUCCESS` | 请求成功 | — |
| `422` | `40000` | `ErrorCode.INVALID_PARAMETER` | 请求参数不合法 | 检查参数格式、必填项与取值范围 |
| `404` | `40400` | `ErrorCode.FILE_NOT_FOUND` | 目标文件不存在 | 确认 `file_id` 是否正确，文件是否已被删除 |
| `409` | `40900` | `ErrorCode.FILE_CONFLICT` | 资源冲突 | 预留，目前未触发 |
| `413` | `41300` | `ErrorCode.FILE_TOO_LARGE` | 上传文件超过大小限制 | 压缩文件或调大 `LEO_MAX_UPLOAD_SIZE_MB` |
| `415` | `41500` | `ErrorCode.UNSUPPORTED_FILE_TYPE` | 文件类型不被允许 | 使用白名单内的扩展名，或调整模块白名单配置 |
| `500` | `50000` | `ErrorCode.STORAGE_ERROR` | 文件存储失败 | 检查磁盘权限与剩余空间 |
| `500` | `50001` | `ErrorCode.INTERNAL_ERROR` | 服务器内部错误 | 携带 `request_id` 反馈给服务端排查 |

## 常见场景

### `41500` 不支持的文件类型

文本文件模块只接受 `.txt` `.md` `.doc` `.docx` `.epub` 五种扩展名。
上传 `.png`、`.pdf`、`.exe` 等类型会返回此错误：

```json
{
  "code": 41500,
  "message": "不支持的文件类型 .png，允许的类型：.doc, .docx, .epub, .md, .txt",
  "data": null,
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```

错误消息中会列出当前生效的白名单，便于客户端提示用户。

### `41300` 文件超过大小限制

```json
{
  "code": 41300,
  "message": "文件大小超过限制，最大允许 50.00 MB",
  "data": null,
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```

### `40000` 参数校验失败

除业务参数不合法外，请求体校验失败时 `data` 会包含 FastAPI 的详细错误列表：

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

## 异常类型与模块

业务异常统一定义在 `app/shared/errors.py`，任何功能模块都可复用：

| 异常类 | `code` | HTTP 状态码 |
| --- | --- | --- |
| `AppError` | `50001` | `500` |
| `InvalidParameterError` | `40000` | `422` |
| `FileTooLargeError` | `41300` | `413` |
| `UnsupportedFileTypeError` | `41500` | `415` |
| `FileNotFoundError` | `40400` | `404` |
| `FileConflictError` | `40900` | `409` |
| `StorageError` | `50000` | `500` |

全局异常处理器位于 `app/main.py`，会自动把上述异常转换为标准响应结构。

## 相关文档

- [快速开始](./getting-started.md)
- [配置说明](./configuration.md)
