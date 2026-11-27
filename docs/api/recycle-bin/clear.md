# 清空回收站

| 属性 | 值 |
| --- | --- |
| 接口地址 | `DELETE /api/v3/recycle-bin` |
| 请求格式 | 无请求体 |
| 响应格式 | `application/json` |
| 认证方式 | 无 |

## 接口描述

彻底删除回收站中的所有项目及其关联文本文件、章节、切句与多音字标注。该操作不可恢复。

## 请求参数

无 Path、Query、Body 参数。可选 Header：`X-Request-ID`。

## 返回字段

### `data`

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `cleared_count` | `integer` | 本次彻底删除的项目数量 |
| `cleared` | `boolean` | 是否已完成清空，固定为 `true` |

## curl

```bash
curl -X DELETE "http://127.0.0.1:58685/api/v3/recycle-bin"
```

## 成功响应示例

```json
{
  "code": 0,
  "message": "回收站已清空，共彻底删除 2 个项目",
  "data": {
    "cleared_count": 2,
    "cleared": true
  },
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```
