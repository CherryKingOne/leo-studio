# 彻底删除项目

物理删除项目及其关联的小说文件、章节、切句与多音字标注数据。

| 属性 | 值 |
| --- | --- |
| **接口地址** | `DELETE /api/v2/projects/{project_id}/hard` |
| **所属模块** | 回收站（`recycle-bin`） |
| **请求格式** | 无请求体 |
| **响应格式** | `application/json` |
| **认证方式** | 无 |
| **幂等性** | 是（重复删除返回 `404`，未进入回收站返回 `422`） |

## 接口描述

从 `projects.csv` 中物理删除项目记录，并调用内部文本文件能力清理磁盘文件、
`novels.csv`、`novel_contents.csv`、`novel_sentences.csv`、`novel_polyphones.csv`
中的关联数据。仅允许删除已移入回收站的项目；正常项目需先调用软删除接口。

## 请求参数

### Path

| 名称 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `project_id` | `string` | **是** | 项目唯一标识 |

### Header

| 名称 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `X-Request-ID` | `string` | 否 | 自定义请求追踪 ID |

## 响应字段

### `data` 对象（`ProjectHardDeleteResponse`）

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `project_id` | `string` | 项目唯一标识 |
| `name` | `string` | 项目名称 |
| `hard_deleted` | `boolean` | 固定为 `true` |

## 代码示例

```bash
curl -X DELETE "http://127.0.0.1:58685/api/v2/projects/9f8c12a4b3e2489c91d8e5f2a1b3c4d5/hard"
```

## 成功响应示例

```json
{
  "code": 0,
  "message": "项目已彻底物理删除",
  "data": {
    "project_id": "9f8c12a4b3e2489c91d8e5f2a1b3c4d5",
    "name": "示例项目",
    "hard_deleted": true
  },
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```

## 失败响应示例

### 404 项目不存在

```json
{
  "code": 40400,
  "message": "项目不存在：9f8c12a4b3e2489c91d8e5f2a1b3c4d5",
  "data": null,
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```

### 422 项目未进入回收站

```json
{
  "code": 40000,
  "message": "项目未处于回收站中：9f8c12a4b3e2489c91d8e5f2a1b3c4d5",
  "data": null,
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```

## 注意事项

- 物理删除不可恢复，请谨慎调用。
- 若关联文件已被手动删除，删除逻辑会忽略 `404` 继续清理数据。