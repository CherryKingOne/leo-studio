# API v2 接口文档

本目录为 **Leo Studio 后端 v2 版本**的接口文档，对应服务前缀 `/api/v2`。
v1 接口保持原样继续提供服务（见 [../v1/README.md](../v1/README.md)），
v2 面向新客户端，已在 `backend/src/modules/v2` 实现。

> v2 路由已实现，接口以「一个接口一个 Markdown 文件」组织，
> 与 `backend/src/modules/v2/router.rs` 保持一致。

## 设计目标

相比 v1，v2 重点解决以下问题：

1. **职责拆分** —— v1 将回收站、章节、多音字等全部压在 `projects` 下，
   v2 按子资源拆分为 `projects`、`recycle-bin`、`chapters`、`polyphones` 平级模块。
2. **文本文件能力内聚** —— 文本文件仅作为内部函数/类被业务模块调用，
   **不再对外暴露** `/api/v2/text-files`，项目创建/追加仍通过 `multipart` 上传文件。
3. **一致的资源语义** —— 统一使用名词复数、HTTP 方法表达动作，减少动词后缀路径。
4. **可预测的列表能力** —— 列表统一支持分页、过滤与排序。
5. **稳定的契约** —— 统一响应包装、错误码、ID 与时间格式，便于多端接入。

## 基本信息

| 项目 | 说明 |
| --- | --- |
| 服务名称 | Leo Studio API Server |
| 基础地址 | `http://127.0.0.1:58685` |
| 业务前缀 | `/api/v2` |
| 数据格式 | `application/json; charset=utf-8` |
| 文件上传格式 | `multipart/form-data` |
| 认证方式 | 无（当前版本未接入鉴权） |
| 默认端口 | `58685`（`LEO_PORT`） |

## API Server 启动

后端以 **API Server 模式**运行，进程启动后对外提供 REST 接口：

```bash
cd backend
cargo run --release
# 监听 http://0.0.0.0:58685
```

常用环境变量：

| 环境变量 | 默认值 | 说明 |
| --- | --- | --- |
| `LEO_HOST` | `0.0.0.0` | 监听地址 |
| `LEO_PORT` | `58685` | 监听端口 |
| `LEO_DATA_DIR` | `.` | 数据根目录 |
| `LEO_CSV_DIR` | `<data>/.leo-studio` | CSV 目录 |
| `LEO_UPLOAD_DIR` | `<data>/storage/uploads/text-files` | 上传目录 |
| `LEO_MAX_UPLOAD_SIZE_MB` | `50` | 全局上传上限 |

健康检查：

```bash
curl http://127.0.0.1:58685/api/v2/health
```

## 统一响应结构

所有业务接口（下载/文件流除外）返回统一 JSON 包装：

```json
{
  "code": 0,
  "message": "success",
  "data": {},
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```

- `request_id` 同时出现在响应体与响应头 `X-Request-ID`。
- 健康检查与根索引接口直接返回业务对象（同样带 `request_id`）。

### 错误码

| HTTP | 业务码 | 含义 |
| --- | --- | --- |
| 422 | 40000 | 参数校验失败 |
| 404 | 40400 | 项目/章节/句子不存在 |
| 409 | 40900 | 资源冲突 |
| 413 | 41300 | 上传文件超过大小限制 |
| 415 | 41500 | 不支持的文件类型 |
| 500 | 50000 | 存储层错误 |
| 500 | 50001 | 服务内部错误 |

## 目录

### 系统（`system`）

| 接口 | 方法 | 文档 |
| --- | --- | --- |
| 健康检查 | `GET /api/v2/health` | [health.md](./system/health.md) |
| 服务索引 | `GET /api/v2` | [index.md](./system/index.md) |

### 项目（`projects`）

| 接口 | 方法 | 文档 |
| --- | --- | --- |
| 创建项目 | `POST /api/v2/projects` | [create.md](./projects/create.md) |
| 查询项目列表 | `GET /api/v2/projects` | [list.md](./projects/list.md) |
| 查询项目详情 | `GET /api/v2/projects/{project_id}` | [get.md](./projects/get.md) |
| 更新项目 | `PATCH /api/v2/projects/{project_id}` | [update.md](./projects/update.md) |
| 删除项目（软删） | `DELETE /api/v2/projects/{project_id}` | [delete.md](./projects/delete.md) |

### 回收站（`recycle-bin`）

| 接口 | 方法 | 文档 |
| --- | --- | --- |
| 查询回收站列表 | `GET /api/v2/projects/recycle-bin` | [list.md](./recycle-bin/list.md) |
| 清空回收站 | `DELETE /api/v2/projects/recycle-bin` | [clear.md](./recycle-bin/clear.md) |
| 恢复项目 | `POST /api/v2/projects/{project_id}/restore` | [restore.md](./recycle-bin/restore.md) |
| 彻底删除项目 | `DELETE /api/v2/projects/{project_id}/hard` | [hard-delete.md](./recycle-bin/hard-delete.md) |

### 章节（`chapters`）

| 接口 | 方法 | 文档 |
| --- | --- | --- |
| 查询章节大纲 | `GET /api/v2/projects/{project_id}/chapters` | [list.md](./chapters/list.md) |
| 追加上传章节 | `POST /api/v2/projects/{project_id}/chapters` | [append.md](./chapters/append.md) |
| 查询章节内容 | `GET /api/v2/projects/{project_id}/chapters/{chapter_id}` | [get.md](./chapters/get.md) |

### 多音字标注（`polyphones`）

| 接口 | 方法 | 文档 |
| --- | --- | --- |
| 查询多音字标注 | `GET /api/v2/projects/{project_id}/sentences/{sentence_id}/polyphones` | [list.md](./polyphones/list.md) |
| 保存多音字标注 | `PUT /api/v2/projects/{project_id}/sentences/{sentence_id}/polyphones` | [save.md](./polyphones/save.md) |
| 清空多音字标注 | `DELETE /api/v2/projects/{project_id}/sentences/{sentence_id}/polyphones` | [delete.md](./polyphones/delete.md) |

## 通用约定

- **ID**：UUID v4 的 32 位无连字符十六进制字符串。
- **时间**：UTC，RFC 3339 序列化。
- **分页**：`page` 默认 1，`page_size` 默认 20、上限 100。
- **命名**：JSON 字段使用 `snake_case`。
- **幂等**：`GET`/`PUT`/`DELETE` 幂等；`POST` 创建类接口不保证幂等。

## 相关文档

- [快速开始](./getting-started.md)
- [v1 接口文档](../v1/README.md)
- [配置说明](../configuration.md)
- [错误码说明](../errors.md)