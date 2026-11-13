# 清空多音字标注

删除指定句子的全部多音字标注。

| 属性 | 值 |
| --- | --- |
| **接口地址** | `DELETE /api/v2/projects/{project_id}/sentences/{sentence_id}/polyphones` |
| **所属模块** | 多音字标注（`polyphones`） |
| **请求格式** | 无请求体 |
| **响应格式** | `application/json` |
| **认证方式** | 无 |
| **幂等性** | 是 |

## 接口描述

先校验项目存在且未删除、句子属于该项目，再删除该句子的全部标注，
返回被删除的数量。若句子无标注，`deleted_count` 为 0。

## 请求参数

### Path

| 名称 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `project_id` | `string` | **是** | 项目唯一标识 |
| `sentence_id` | `string` | **是** | 切句唯一标识 |

### Header

| 名称 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `X-Request-ID` | `string` | 否 | 自定义请求追踪 ID |

## 响应字段

### `data` 对象（`SentencePolyphoneDeleteResponse`）

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `sentence_id` | `string` | 切句唯一标识 |
| `deleted_count` | `integer` | 被删除的标注数量 |
| `deleted` | `boolean` | 固定为 `true` |

## 代码示例

```bash
curl -X DELETE "http://127.0.0.1:58685/api/v2/projects/9f8c12a4b3e2489c91d8e5f2a1b3c4d5/sentences/s_0001/polyphones"
```

## 成功响应示例

```json
{
  "code": 0,
  "message": "已成功清空多音字标注，共删除 1 条",
  "data": {
    "sentence_id": "s_0001",
    "deleted_count": 1,
    "deleted": true
  },
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```

## 失败响应示例

### 404 句子不存在

```json
{
  "code": 40400,
  "message": "句子不存在：s_9999",
  "data": null,
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```