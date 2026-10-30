# 更新项目名称（重命名）

根据 `project_id` 更新指定项目的名称（剧目重命名）。

| 属性 | 值 |
| --- | --- |
| **接口地址** | `PATCH /api/v1/projects/{project_id}`（同时支持 `PUT`） |
| **所属模块** | 剧目与项目管理（`app/modules/projects/`） |
| **请求格式** | `application/json` |
| **响应格式** | `application/json` |
| **认证方式** | 无（当前版本未接入鉴权） |
| **幂等性** | 是 |

## 接口描述

用于修改指定活跃项目的名称：

1. 仅支持修改项目名称 `name`（剧目重命名，1~255 字符，禁止纯空白）；
2. 成功更新后，系统会自动刷新项目的 `updated_at` 时间戳；
3. 处于回收站中（软删除状态）的项目不可更新，会返回 `404`。

## 路径参数

| 名称 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `project_id` | `string` | **是** | 项目唯一标识，32 位十六进制字符串 |

## 请求字段

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `name` | `string` | **是** | 项目新名称（重命名），长度为 1~255 个字符，首尾空白会自动清除 |

## 响应字段

### `data` 对象（`ProjectResponse`）

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `project_id` | `string` | 项目唯一标识，标准 UUID（UUID4） |
| `name` | `string` | 更新后的项目名称 |
| `file_id` | `string` | 关联文本文件唯一标识 (UUID4) |
| `filename` | `string` | 原始清洗后文件名 |
| `status` | `string` | 制作状态 |
| `chapter_count` | `integer` | 章节总数 |
| `total_words` | `integer` | 有效总字数 |
| `created_at` | `string` | 创建时间，ISO 8601 格式 |
| `updated_at` | `string` | 更新时间，ISO 8601 格式（已刷新） |
| `is_deleted` | `boolean` | 是否处于回收站（软删除状态） |
| `deleted_at` | `string \| null` | 软删除时间戳，正常活跃项目中为 `null` |

## 成功示例

### 请求

```bash
curl -X PATCH "http://127.0.0.1:8000/api/v1/projects/915c2735cb2a40a18b9293ae86bae158" \
  -H "Content-Type: application/json" \
  -d '{"name": "测试你好"}'
```

### 响应

```json
{
  "code": 0,
  "message": "项目更新成功",
  "data": {
    "project_id": "9f8c12a4b3e2489c91d8e5f2a1b3c4d5",
    "name": "诡秘之主：第一部 小丑",
    "file_id": "a1b2c3d4e5f67890123456789abcdef0",
    "filename": "诡秘之主.txt",
    "status": "制作中",
    "chapter_count": 12,
    "total_words": 45820,
    "created_at": "2026-10-27T08:55:00.000000+00:00",
    "updated_at": "2026-10-30T03:45:00.000000+00:00",
    "is_deleted": false,
    "deleted_at": null
  },
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```

## 失败示例

### 项目不存在或已移入回收站 (404)

```json
{
  "code": 40400,
  "message": "项目不存在或已移入回收站：9f8c12a4b3e2489c91d8e5f2a1b3c4d5",
  "data": null,
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```

### 项目名称为空 (422)

```json
{
  "code": 40000,
  "message": "项目名称不能为空",
  "data": null,
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```

### 项目名称超过 255 字符 (422)

```json
{
  "code": 40000,
  "message": "项目名称长度不能超过 255 个字符",
  "data": null,
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```

## 前端接入

`desktop/` 的两处剧目重命名入口都走本接口。后端未开 CORS，浏览器不直连，
统一由服务端动作发起请求：

| 页面 | 入口 | 调用链 |
| --- | --- | --- |
| `/project`（我的剧目） | 卡片菜单「重命名」后的行内输入框 | `ProjectLibrary.submitRename` → `renameProjectAction` → `renameProject` |
| `/project/{project_id}`（剧目看板） | 顶栏标题双击或铅笔图标 | `HubHeader.commit` → 同上 |

- 请求期间前端先乐观替换标题：成功即作废 `projects` 缓存标签并刷新路由，
  书架、侧栏与看板同步换新名；失败（404、超过 255 字符等）则回滚标题，
  并把响应的 `message` 原样提示给用户。
- 原型 mock 剧目（`desktop/lib/projects.ts` 的 `initialProjects`）没有后端记录，
  看板顶栏改名只改本地标题，不发请求。
