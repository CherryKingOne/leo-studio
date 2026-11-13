# 健康检查

检查 API Server 是否存活，返回服务状态、应用名与版本号。

| 属性 | 值 |
| --- | --- |
| **接口地址** | `GET /api/v2/health` |
| **所属模块** | 系统（`system`） |
| **请求格式** | 无请求体 |
| **响应格式** | `application/json` |
| **认证方式** | 无 |
| **幂等性** | 是 |

## 接口描述

用于探活、负载均衡健康检查与客户端启动自检。接口不读取业务数据，
直接返回进程内置的应用名与版本号。

## 请求参数

### Header

| 名称 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `X-Request-ID` | `string` | 否 | 自定义请求追踪 ID，未传则由服务端生成 |

### 路径 / 查询 / Body

无。

## 响应字段

该接口不使用统一 `ApiResponse` 包装，直接返回业务对象；响应体仍会注入 `request_id`。

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `status` | `string` | 固定为 `ok` |
| `app` | `string` | 应用名称（`LEO_APP_NAME`） |
| `version` | `string` | 应用版本（`LEO_APP_VERSION`） |
| `request_id` | `string` | 请求追踪 ID |

## 代码示例

```bash
curl -s "http://127.0.0.1:58685/api/v2/health"
```

## 成功响应示例

```json
{
  "status": "ok",
  "app": "Leo Studio API",
  "version": "0.1.0",
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```