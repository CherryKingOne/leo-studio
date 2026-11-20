# 清空回收站

彻底删除回收站中的所有项目及其关联数据。

| 属性 | 值 |
| --- | --- |
| **接口地址** | `DELETE /api/v2/projects/recycle-bin` |
| **所属模块** | 回收站（`recycle-bin`） |
| **请求格式** | 无请求体 |
| **响应格式** | `application/json` |
| **认证方式** | 无 |
| **幂等性** | 是 |

## 接口描述

遍历回收站中的全部项目，逐个物理删除项目记录并清理关联的小说文件、章节、
切句与多音字标注，返回被清理的项目数量。回收站为空时返回 `cleared_count = 0`。

## 请求参数

### Header

| 名称 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `X-Request-ID` | `string` | 否 | 自定义请求追踪 ID |

### Path / Query / Body

无。

## 响应字段

### `data` 对象（`ProjectClearRecycleBinResponse`）

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `cleared_count` | `integer` | 被彻底删除的项目数量 |
| `cleared` | `boolean` | 固定为 `true` |

## 代码示例

```bash
curl -X DELETE "http://127.0.0.1:58685/api/v2/projects/recycle-bin"
```

## 成功响应示例

```json
{
  "code": 0,
  "message": "回收站已清空，共彻底删除 3 个项目",
  "data": {
    "cleared_count": 3,
    "cleared": true
  },
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```

## 注意事项

- 该操作不可恢复。
- 若某个项目关联文件已不存在，会忽略 `404` 继续清理其余项目。