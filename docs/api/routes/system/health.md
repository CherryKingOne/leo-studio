# 系统模块健康检查

系统模块健康检查接口，挂载在 `/api/v1/system` 前缀下。

| 属性 | 值 |
| --- | --- |
| **接口地址** | `GET /api/v1/system/health` |
| **所属模块** | 系统（`app/modules/system/`） |
| **请求格式** | 无请求体 |
| **响应格式** | `application/json` |
| **认证方式** | 无（当前版本未接入鉴权） |
| **幂等性** | 是（只读操作） |

## 接口描述

用于负载均衡器、容器编排平台（如 Kubernetes）或监控系统探活。
与根路径的 `/health` 接口功能一致，仅路径不同。

## 响应字段

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `status` | `string` | 固定为 `"ok"` |
| `app` | `string` | 应用名称 |
| `version` | `string` | 应用版本号 |

## 成功示例

### 请求

```bash
curl -X GET "http://127.0.0.1:8000/api/v1/system/health"
```

```python
import httpx

response = httpx.get("http://127.0.0.1:8000/api/v1/system/health")
print(response.json())
```

### 响应

`HTTP/1.1 200 OK`

```json
{
  "status": "ok",
  "app": "Leo Studio",
  "version": "0.1.0"
}
```

## 注意事项

- **与根路径 health 的区别**：本接口挂载在 `/api/v1/system/health`，
  另一个同名接口在根路径 `/health`，两者返回内容一致。
  生产环境建议选择与业务接口同前缀的版本，便于统一网关路由管理。

## 相关接口

- [应用健康检查](../root/health.md) —— 挂载在根路径 `/health` 的健康检查
- [服务信息](../root/index.md) —— 服务基本信息与接口索引
