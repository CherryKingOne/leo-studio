# API v1 接口文档

本目录为 **Leo Studio 后端 v1 版本**的接口文档，对应服务前缀 `/api/v1`。
文档依据 `backend/src` 下的 Axum 路由与数据结构编写，一个接口对应一个 Markdown 文件。

## 基本信息

| 项目 | 说明 |
| --- | --- |
| 基础地址 | `http://127.0.0.1:58685` |
| 业务前缀 | `/api/v1` |
| 数据格式 | `application/json; charset=utf-8` |
| 文件上传格式 | `multipart/form-data` |
| 认证方式 | 无（当前版本未接入鉴权） |
| 默认端口 | `58685`（`LEO_PORT`） |

## 统一响应结构

除文件下载接口外，所有业务接口均返回统一 JSON 包装：

```json
{
  "code": 0,
  "message": "success",
  "data": {},
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```

`request_id` 同时出现在响应体与响应头 `X-Request-ID`。健康检查与根路径接口
不使用该包装，直接返回业务对象（同样带 `request_id`）。

### 错误码

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

### 系统（`system`）

| 接口 | 方法 | 文档 |
| --- | --- | --- |
| 健康检查 | `GET /health`、`GET /api/v1/system/health` | [health.md](./system/health.md) |
| 服务索引 | `GET /` | [index.md](./system/index.md) |

### 文本文件（`text-files`）

| 接口 | 方法 | 文档 |
| --- | --- | --- |
| 上传文本文件 | `POST /api/v1/text-files`、`/upload` | [upload.md](./text-files/upload.md) |
| 查询文件列表 | `GET /api/v1/text-files`、`/upload` | [list.md](./text-files/list.md) |
| 查询文件详情 | `GET /api/v1/text-files/{file_id}` | [get.md](./text-files/get.md) |
| 预览文件内容 | `GET /api/v1/text-files/{file_id}/content` | [content.md](./text-files/content.md) |
| 下载文件 | `GET /api/v1/text-files/{file_id}/download` | [download.md](./text-files/download.md) |
| 删除文件 | `DELETE /api/v1/text-files/{file_id}` | [delete.md](./text-files/delete.md) |
| 查询切句列表 | `GET /api/v1/text-files/{file_id}/sentences` | [sentences.md](./text-files/sentences.md) |

### 项目（`projects`）

| 接口 | 方法 | 文档 |
| --- | --- | --- |
| 创建项目并上传 | `POST /api/v1/projects`、`/upload` | [create.md](./projects/create.md) |
| 查询项目列表 | `GET /api/v1/projects` | [list.md](./projects/list.md) |
| 查询项目详情 | `GET /api/v1/projects/{project_id}` | [get.md](./projects/get.md) |
| 更新项目名称 | `PATCH/PUT /api/v1/projects/{project_id}` | [update.md](./projects/update.md) |
| 删除项目（回收站） | `DELETE /api/v1/projects/{project_id}` | [delete.md](./projects/delete.md) |
| 恢复项目 | `POST /api/v1/projects/{project_id}/restore` | [restore.md](./projects/restore.md) |
| 彻底删除项目 | `DELETE /api/v1/projects/{project_id}/hard` | [hard-delete.md](./projects/hard-delete.md) |
| 查询回收站列表 | `GET /api/v1/projects/recycle-bin` | [recycle-bin-list.md](./projects/recycle-bin-list.md) |
| 清空回收站 | `DELETE /api/v1/projects/recycle-bin` | [recycle-bin-clear.md](./projects/recycle-bin-clear.md) |
| 查询章节大纲 | `GET /api/v1/projects/{project_id}/chapters` | [chapters-list.md](./projects/chapters-list.md) |
| 追加上传章节 | `POST /api/v1/projects/{project_id}/chapters`、`/chapters/upload` | [chapters-append.md](./projects/chapters-append.md) |
| 查询章节内容 | `GET /api/v1/projects/{project_id}/chapters/{chapter_id}`、`/content` | [chapter-content.md](./projects/chapter-content.md) |
| 查询多音字标注 | `GET /api/v1/projects/{project_id}/sentences/{sentence_id}/polyphones` | [polyphones-list.md](./projects/polyphones-list.md) |
| 保存多音字标注 | `PUT /api/v1/projects/{project_id}/sentences/{sentence_id}/polyphones` | [polyphones-save.md](./projects/polyphones-save.md) |
| 清空多音字标注 | `DELETE /api/v1/projects/{project_id}/sentences/{sentence_id}/polyphones` | [polyphones-delete.md](./projects/polyphones-delete.md) |

## 相关文档

- [快速开始](../getting-started.md)
- [配置说明](../configuration.md)
- [错误码说明](../errors.md)
- [v2 接口文档](../v2/README.md)（预留）