# 服务索引

返回服务的基本信息、文档入口与模块清单，便于前端或客户端自动发现后端能力。

| 属性 | 值 |
| --- | --- |
| **接口地址** | `GET /` |
| **所属模块** | 系统（`src/modules/system.rs`） |
| **请求格式** | 无请求体 |
| **响应格式** | `application/json` |
| **认证方式** | 无（当前版本未接入鉴权） |
| **幂等性** | 是 |

## 接口描述

服务根路径，返回应用名、版本、文档地址、健康检查地址、业务前缀与模块列表。
可用于前端环境探测或运维脚本快速确认后端版本与能力范围。

## 请求参数

### Header

| 名称 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `X-Request-ID` | `string` | 否 | 自定义请求追踪 ID，未传则由服务端生成 |

### 路径参数 / 查询参数 / Body

无。

## 响应字段

该接口**不使用**统一 `ApiResponse` 包装，直接返回对象；
响应体中仍会由中间件注入 `request_id` 字段。

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `app` | `string` | 应用名称，默认 `Leo Studio API` |
| `version` | `string` | 应用版本，默认 `0.1.0` |
| `docs` | `string` | 文档入口路径，固定为 `/docs` |
| `health` | `string` | 健康检查路径，固定为 `/health` |
| `api_prefix` | `string` | 业务接口前缀，固定为 `/api/v1` |
| `modules` | `string[]` | 已注册模块：`text-files`、`projects`、`system` |
| `request_id` | `string` | 请求追踪 ID |

## 代码示例

### curl

```bash
curl -s "http://127.0.0.1:58685/"
```

## 成功响应示例

`HTTP/1.1 200 OK`

```json
{
  "app": "Leo Studio API",
  "version": "0.1.0",
  "docs": "/docs",
  "health": "/health",
  "api_prefix": "/api/v1",
  "modules": ["text-files", "projects", "system"],
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```

## 注意事项

- 当前代码未注册 `/docs` 静态文档路由，`docs` 字段为预留提示。
- 模块清单用于展示，实际接口以本目录下各模块文档为准。