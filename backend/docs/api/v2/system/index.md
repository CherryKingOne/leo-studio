# 服务索引

返回 API Server 的基本信息、业务前缀与模块清单，便于客户端自动发现能力。

| 属性 | 值 |
| --- | --- |
| **接口地址** | `GET /api/v2` |
| **所属模块** | 系统（`system`） |
| **请求格式** | 无请求体 |
| **响应格式** | `application/json` |
| **认证方式** | 无 |
| **幂等性** | 是 |

## 接口描述

返回应用名、版本、业务前缀与已注册模块，可用于环境探测或运维脚本。

## 请求参数

### Header

| 名称 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `X-Request-ID` | `string` | 否 | 自定义请求追踪 ID |

### 路径 / 查询 / Body

无。

## 响应字段

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `app` | `string` | 应用名称 |
| `version` | `string` | 应用版本 |
| `api_prefix` | `string` | 固定为 `/api/v2` |
| `modules` | `string[]` | 模块清单：`system`、`projects`、`recycle-bin`、`chapters`、`polyphones` |
| `request_id` | `string` | 请求追踪 ID |

## 代码示例

```bash
curl -s "http://127.0.0.1:58685/api/v2"
```

## 成功响应示例

```json
{
  "app": "Leo Studio API",
  "version": "0.1.0",
  "api_prefix": "/api/v2",
  "modules": ["system", "projects", "recycle-bin", "chapters", "polyphones"],
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```