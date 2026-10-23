# 服务信息

返回服务基本信息与接口索引。

| 属性 | 值 |
| --- | --- |
| **接口地址** | `GET /` |
| **所属模块** | 系统（`app/modules/system/`） |
| **请求格式** | 无请求体 |
| **响应格式** | `application/json` |
| **认证方式** | 无（当前版本未接入鉴权） |
| **幂等性** | 是（只读操作） |

## 接口描述

返回应用名称、版本号、文档地址以及可用的功能模块列表。

## 响应字段

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `app` | `string` | 应用名称 |
| `version` | `string` | 应用版本号 |
| `docs` | `string` | API 文档地址 |
| `health` | `string` | 健康检查接口地址 |
| `api_prefix` | `string` | API 版本前缀 |
| `modules` | `string[]` | 已注册的功能模块列表 |

## 成功示例

### 请求

```bash
curl -X GET "http://127.0.0.1:8000/"
```

```python
import httpx

response = httpx.get("http://127.0.0.1:8000/")
print(response.json())
```

### 响应

`HTTP/1.1 200 OK`

```json
{
  "app": "Leo Studio",
  "version": "0.1.0",
  "docs": "/docs",
  "health": "/health",
  "api_prefix": "/api/v1",
  "modules": ["text-files", "system"]
}
```

## 相关接口

- [健康检查](./health.md) —— 应用健康检查接口
- [文本文件管理](../text-files/list.md) —— 文本文件管理模块接口
