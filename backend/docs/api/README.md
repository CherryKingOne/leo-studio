# API 接口文档

Leo Studio 后端服务的 HTTP 接口文档。后端使用 **Rust + Axum** 实现，
对外保持与旧版 Python 服务一致的 URL、请求参数、响应字段与业务错误码。

## 基本信息

| 项目 | 说明 |
| --- | --- |
| 服务名称 | Leo Studio API |
| 基础地址 | `http://127.0.0.1:58685` |
| 业务接口前缀 | `/api/v1` |
| 文本文件接口前缀 | `/api/v1/text-files` |
| 项目接口前缀 | `/api/v1/projects` |
| 数据格式 | `application/json; charset=utf-8` |
| 文件上传格式 | `multipart/form-data` |
| 健康检查 | `/health`、`/api/v1/system/health` |
| 服务索引 | `/` |

## 统一响应结构

所有业务接口（下载接口除外）均返回统一的 JSON 包装结构，
`request_id` 同时出现在响应头 `X-Request-ID` 与响应体中。

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
  "message": "不支持的文件类型 .exe，允许的类型：.txt",
  "data": null,
  "request_id": "…"
}
```

HTTP 状态码与业务错误码的映射：

| HTTP | 业务码 | 含义 |
| --- | --- | --- |
| 422 | 40000 | 参数校验失败 |
| 404 | 40400 | 文件或项目不存在 |
| 409 | 40900 | 资源冲突 |
| 413 | 41300 | 上传文件超过大小限制 |
| 415 | 41500 | 不支持的文件类型 |
| 500 | 50000 | 存储层错误 |
| 500 | 50001 | 服务内部错误 |

## 目录

- [快速开始](getting-started.md)
- [配置说明](configuration.md)
- [错误码说明](errors.md)
- [项目接口](projects/)
- [文本文件接口](text-files/)