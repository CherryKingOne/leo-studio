# 健康检查

检查后端服务是否存活，返回服务状态、应用名与版本号。

| 属性 | 值 |
| --- | --- |
| **接口地址** | `GET /health`（别名：`GET /api/v1/system/health`） |
| **所属模块** | 系统（`src/modules/system.rs`） |
| **请求格式** | 无请求体 |
| **响应格式** | `application/json` |
| **认证方式** | 无（当前版本未接入鉴权） |
| **幂等性** | 是 |

## 接口描述

用于探活、负载均衡健康检查与前端启动自检。接口不读取任何业务数据，
直接返回进程内置的应用名与版本号。`/health` 与 `/api/v1/system/health`
两个路径行为完全一致。

## 请求参数

### Header

| 名称 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `X-Request-ID` | `string` | 否 | 自定义请求追踪 ID，未传则由服务端生成 |

### 路径参数 / 查询参数 / Body

无。

## 响应字段

该接口**不使用**统一 `ApiResponse` 包装，直接返回业务对象；
响应体中仍会由中间件注入 `request_id` 字段。

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `status` | `string` | 固定为 `ok` |
| `app` | `string` | 应用名称，默认 `Leo Studio API`（`LEO_APP_NAME`） |
| `version` | `string` | 应用版本，默认 `0.1.0`（`LEO_APP_VERSION`） |
| `request_id` | `string` | 请求追踪 ID，同时写入响应头 `X-Request-ID` |

## 代码示例

### curl

```bash
curl -s "http://127.0.0.1:58685/health"
```

### Python

```python
import httpx

resp = httpx.get("http://127.0.0.1:58685/health")
print(resp.json())
# {'status': 'ok', 'app': 'Leo Studio API', 'version': '0.1.0', 'request_id': '...'}
```

## 成功响应示例

`HTTP/1.1 200 OK`

```json
{
  "status": "ok",
  "app": "Leo Studio API",
  "version": "0.1.0",
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```

## 失败响应示例

健康检查接口在正常提供服务时不会返回业务错误；若进程未启动或端口不可达，
连接层会直接失败（无 HTTP 响应）。

## 注意事项

- 该接口仅表示进程存活，不代表 CSV 存储或磁盘可写，业务可用性请结合具体业务接口判断。
- `request_id` 既出现在响应体，也出现在响应头 `X-Request-ID`。