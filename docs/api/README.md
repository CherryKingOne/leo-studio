# Leo Studio API v3 接口文档

本文档以当前 Rust 代码实际挂载的路由为准，路径为 `plugins/interfaces/api/src`。当前 API 只暴露 `/api/v3` 项目管理相关接口，共 27 个。

## 基本信息

| 项目 | 说明 |
| --- | --- |
| 基础地址 | `http://127.0.0.1:58685` |
| 业务前缀 | `/api/v3` |
| 数据格式 | `application/json; charset=utf-8` |
| 文件上传 | `multipart/form-data` |
| 认证方式 | 无 |
| 请求追踪 | 可选 Header `X-Request-ID`，响应头与 JSON 响应体返回 `request_id` |

## 目录约定

```text
docs/api/
├── README.md
├── route-migration-plan.md
├── projects/          # 项目管理（5 个接口）
├── project-chapters/  # 项目章节管理（4 个接口）
├── project-dialogues/   # 项目台词管理（4 个接口）
├── project-characters/ # 项目角色管理（5 个接口）
├── project-polyphones/ # 项目多音字管理（5 个接口）
└── recycle-bin/       # 回收站管理（4 个接口）
```

每个 Markdown 文件对应一个「HTTP 方法 + 路径」接口，包含请求参数、返回字段与 `curl` 示例。

## 统一响应

业务接口返回统一 JSON 包装：

```json
{
  "code": 0,
  "message": "success",
  "data": {},
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `code` | `integer` | 业务码，`0` 表示成功 |
| `message` | `string` | 业务消息 |
| `data` | `object/array/null` | 业务数据，失败时为 `null` 或错误详情 |
| `request_id` | `string` | 请求追踪 ID，由中间件写入 |

## 项目管理（5）

| 方法 | 路径 | 文档 |
| --- | --- | --- |
| POST | `/api/v3/projects` | [projects/create.md](projects/create.md) |
| GET | `/api/v3/projects` | [projects/list.md](projects/list.md) |
| GET | `/api/v3/projects/{project_id}` | [projects/get.md](projects/get.md) |
| PATCH | `/api/v3/projects/{project_id}` | [projects/update.md](projects/update.md) |
| DELETE | `/api/v3/projects/{project_id}` | [projects/delete.md](projects/delete.md) |

## 项目章节管理（4）

| 方法 | 路径 | 文档 |
| --- | --- | --- |
| GET | `/api/v3/projects/{project_id}/chapters` | [project-chapters/list.md](project-chapters/list.md) |
| POST | `/api/v3/projects/{project_id}/chapters` | [project-chapters/append.md](project-chapters/append.md) |
| GET | `/api/v3/projects/{project_id}/chapters/{chapter_id}` | [project-chapters/get.md](project-chapters/get.md) |
| DELETE | `/api/v3/projects/{project_id}/chapters/{chapter_id}` | [project-chapters/delete.md](project-chapters/delete.md) |

## 项目台词管理（4）

| 方法 | 路径 | 文档 |
| --- | --- | --- |
| GET | `/api/v3/projects/{project_id}/chapters/{chapter_id}/dialogues` | [project-dialogues/list.md](project-dialogues/list.md) |
| GET | `/api/v3/projects/{project_id}/chapters/{chapter_id}/dialogues/{dialogue_id}` | [project-dialogues/get.md](project-dialogues/get.md) |
| PUT | `/api/v3/projects/{project_id}/chapters/{chapter_id}/dialogues/{dialogue_id}/speaker` | [project-dialogues/bind-speaker.md](project-dialogues/bind-speaker.md) |
| DELETE | `/api/v3/projects/{project_id}/chapters/{chapter_id}/dialogues/{dialogue_id}/speaker` | [project-dialogues/unbind-speaker.md](project-dialogues/unbind-speaker.md) |

## 项目角色管理（5）

| 方法 | 路径 | 文档 |
| --- | --- | --- |
| POST | `/api/v3/projects/{project_id}/characters` | [project-characters/create.md](project-characters/create.md) |
| GET | `/api/v3/projects/{project_id}/characters` | [project-characters/list.md](project-characters/list.md) |
| GET | `/api/v3/projects/{project_id}/characters/{character_id}` | [project-characters/get.md](project-characters/get.md) |
| PATCH | `/api/v3/projects/{project_id}/characters/{character_id}` | [project-characters/update.md](project-characters/update.md) |
| DELETE | `/api/v3/projects/{project_id}/characters/{character_id}` | [project-characters/delete.md](project-characters/delete.md) |

## 项目多音字管理（5）

| 方法 | 路径 | 文档 |
| --- | --- | --- |
| GET | `/api/v3/projects/{project_id}/chapters/{chapter_id}/polyphones` | [project-polyphones/chapter-list.md](project-polyphones/chapter-list.md) |
| GET | `/api/v3/projects/{project_id}/chapters/{chapter_id}/sentences/{sentence_id}/polyphones` | [project-polyphones/sentence-list.md](project-polyphones/sentence-list.md) |
| PUT | `/api/v3/projects/{project_id}/chapters/{chapter_id}/sentences/{sentence_id}/polyphones` | [project-polyphones/sentence-save.md](project-polyphones/sentence-save.md) |
| DELETE | `/api/v3/projects/{project_id}/chapters/{chapter_id}/sentences/{sentence_id}/polyphones` | [project-polyphones/sentence-delete.md](project-polyphones/sentence-delete.md) |
| DELETE | `/api/v3/projects/{project_id}/chapters/{chapter_id}/sentences/{sentence_id}/polyphones/items/{char_index}` | [project-polyphones/item-delete.md](project-polyphones/item-delete.md) |

## 回收站管理（4）

| 方法 | 路径 | 文档 |
| --- | --- | --- |
| GET | `/api/v3/recycle-bin` | [recycle-bin/list.md](recycle-bin/list.md) |
| DELETE | `/api/v3/recycle-bin` | [recycle-bin/clear.md](recycle-bin/clear.md) |
| POST | `/api/v3/recycle-bin/{project_id}/restore` | [recycle-bin/restore.md](recycle-bin/restore.md) |
| DELETE | `/api/v3/recycle-bin/{project_id}` | [recycle-bin/hard-delete.md](recycle-bin/hard-delete.md) |

## 通用约定

- ID：UUID v4 的 32 位无连字符十六进制字符串。
- 时间：UTC，RFC 3339 格式。
- 分页：`page` 默认 `1`；`page_size` 默认 `20`，范围 `1~100`。
- JSON 字段：`snake_case`。
- 上传文件：默认允许 `.txt`，可通过 `LEO_TEXT_FILES_ALLOWED_EXTENSIONS` 调整。
- 上传大小：默认最大 50 MB，可通过 `LEO_MAX_UPLOAD_SIZE_MB` 调整。

## 错误码

| HTTP | 业务码 | 含义 |
| --- | --- | --- |
| 422 | 40000 | 参数校验失败 |
| 404 | 40400 | 文件、项目、章节或句子不存在 |
| 409 | 40900 | 资源冲突 |
| 413 | 41300 | 上传文件超过大小限制 |
| 415 | 41500 | 不支持的文件类型 |
| 500 | 50000 | 存储层错误 |
| 500 | 50001 | 服务内部错误 |
