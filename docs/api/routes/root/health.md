# 应用健康检查

应用级健康检查接口，返回服务运行状态。

| 属性 | 值 |
| --- | --- |
| **接口地址** | `GET /health` |
| **所属模块** | 系统（`app/modules/system/`） |
| **请求格式** | 无请求体 |
| **响应格式** | `application/json` |
| **认证方式** | 无（当前版本未接入鉴权） |
| **幂等性** | 是（只读操作） |

## 接口描述

用于负载均衡器、容器编排平台（如 Kubernetes）或监控系统探活。
仅返回服务是否正常运行，不涉及依赖服务（如数据库、缓存）的健康状态。

## 响应字段

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `status` | `string` | 固定为 `"ok"` |
| `app` | `string` | 应用名称 |
| `version` | `string` | 应用版本号 |

## 成功示例

### 请求

```bash
curl -X GET "http://127.0.0.1:8000/health"
```

```python
import httpx

response = httpx.get("http://127.0.0.1:8000/health")
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

- **与系统模块 health 接口的区别**：本接口挂载在根路径 `/health`，
  系统模块还有一个同名的 `/api/v1/system/health`，两者返回内容一致，
  仅路径不同，可根据部署架构选择合适的探测地址。
- **探活建议**：生产环境建议设置 3~5 秒超时，连续 3 次失败判定为不可用。

## 相关接口

- [服务信息](./index.md) —— 服务基本信息与接口索引
- [系统模块健康检查](../system/health.md) —— 挂载在 `/api/v1/system/health` 的健康检查
