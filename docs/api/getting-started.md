# 快速开始

本文档介绍如何在本地启动 Leo Studio 后端服务并完成第一次文本文件上传。

## 环境要求

| 组件 | 版本要求 | 说明 |
| --- | --- | --- |
| Python | `>= 3.13` | 由 `.python-version` 锁定 |
| uv | 最新版 | Python 包与虚拟环境管理工具 |

> 项目统一使用 [uv](https://docs.astral.sh/uv/) 管理依赖与虚拟环境，
> 避免手工创建 venv 与安装依赖带来的版本漂移。

## 安装依赖

```bash
# 安装 uv（若尚未安装）
# Windows PowerShell
irm https://astral.sh/uv/install.ps1 | iex
# macOS / Linux
curl -LsSf https://astral.sh/uv/install.sh | sh

# 创建虚拟环境并安装全部依赖（含开发依赖）
uv sync --extra dev
```

执行完成后项目根目录会生成 `.venv/` 目录。

## 启动服务

```bash
# 方式一：使用 uvicorn（推荐，支持热重载）
uv run uvicorn app.main:app --reload --host 0.0.0.0 --port 8000

# 方式二：使用项目脚本入口
uv run leo-studio
```

启动成功后可访问：

| 地址 | 说明 |
| --- | --- |
| <http://127.0.0.1:8000/docs> | Swagger UI 交互式文档 |
| <http://127.0.0.1:8000/redoc> | ReDoc 文档 |
| <http://127.0.0.1:8000/openapi.json> | OpenAPI 3.1 描述文件 |
| <http://127.0.0.1:8000/health> | 健康检查 |

## 第一次上传

```bash
# 在项目根目录创建一个测试文件
echo "Hello Leo Studio" > hello.txt

# 上传
curl -X POST "http://127.0.0.1:8000/api/v1/text-files/upload" \
  -F "file=@./hello.txt" \
  -F "description=第一个测试文件"
```

返回：

```json
{
  "code": 0,
  "message": "文件上传成功",
  "data": {
    "file_id": "9f2c1a8b7d6e5f4a3b2c1d0e9f8a7b6c",
    "filename": "hello.txt",
    "stored_name": "9f2c1a8b7d6e5f4a3b2c1d0e9f8a7b6c.txt",
    "extension": ".txt",
    "size": 17,
    "size_human": "17 B",
    "content_type": "text/plain",
    "category": "plain_text",
    "uploaded_at": "2026-10-22T14:32:18.451092+00:00",
    "download_url": "/api/v1/text-files/9f2c1a8b7d6e5f4a3b2c1d0e9f8a7b6c/download",
    "description": "第一个测试文件"
  },
  "request_id": "b3f1c2d4e5a60718293a4b5c6d7e8f90"
}
```

使用返回的 `file_id` 即可下载：

```bash
curl -X GET "http://127.0.0.1:8000/api/v1/text-files/9f2c1a8b7d6e5f4a3b2c1d0e9f8a7b6c/download" \
  -o downloaded.txt
```

也可以直接在线预览文本内容：

```bash
curl -X GET "http://127.0.0.1:8000/api/v1/text-files/9f2c1a8b7d6e5f4a3b2c1d0e9f8a7b6c/content"
```

> **注意**：文本文件接口只接受 `.txt` `.md` `.doc` `.docx` `.epub` 五种扩展名。
> 上传图片、视频等其它二进制文件会返回 `415`。

## 运行测试

```bash
# 运行全部测试
uv run pytest

# 查看覆盖率
uv run pytest --cov=app --cov-report=term-missing

# 代码检查与格式化
uv run ruff check app tests
uv run ruff format app tests

# 命名规范检查（禁止以下划线开头的函数、方法与类名）
uv run python scripts/check_no_underscore_prefix.py
```

测试使用独立的临时目录作为上传目录，不会污染 `storage/uploads/` 中的数据。

### 命名规范

项目**禁止**使用以下划线 `_` 开头的函数、方法与类名：

```python
# fmt: off
# 禁止
def _helper(): ...
class _Helper: ...
class Service:
    def _method(self): ...

# 正确
def helper(): ...
class Helper: ...
class Service:
    def method(self): ...
# fmt: on
```

唯一豁免是双下划线包裹的魔术方法（`__init__`、`__repr__` 等），它们属于 Python
协议保留名。Pydantic 的验证器通过装饰器注册，重命名不影响其生效。

规范由 `scripts/check_no_underscore_prefix.py` 自动检查，并已纳入测试
（`tests/test_naming_convention.py`）。Cline 的项目级 Skill 位于
`.cline/skills/no-underscore-prefix/`，用于在编写代码时持续遵守该约定。

## 项目结构

项目采用**功能优先**的模块组织方式：按业务功能划分目录，
每个功能模块内部自包含路由、Schema、领域模型与业务服务，
模块之间互不感知，新增功能时新建模块目录即可。

```
leo-studio/
├── app/
│   ├── main.py                  # 应用入口：中间件、异常处理、路由注册
│   ├── router.py                # 路由汇总：按功能模块聚合
│   ├── modules/                 # 业务功能模块（按功能划分）
│   │   ├── text_files/          # 文本文件功能模块
│   │   │   ├── constants.py     #   模块常量、扩展名白名单与 MIME 映射
│   │   │   ├── config.py        #   模块独立配置
│   │   │   ├── models.py        #   领域模型与分类规则
│   │   │   ├── schemas.py       #   请求与响应 Schema
│   │   │   ├── service.py       #   存储业务逻辑
│   │   │   ├── dependencies.py  #   依赖注入提供者
│   │   │   └── router.py        #   HTTP 路由定义
│   │   └── system/              # 系统功能模块
│   │       └── router.py        #   健康检查与服务信息
│   ├── shared/                  # 跨模块共享的基础设施
│   │   ├── config.py            #   全局配置
│   │   ├── errors.py            #   统一错误定义
│   │   ├── middleware.py        #   request_id 中间件
│   │   └── response.py          #   统一响应结构
│   └── ...
├── docs/
│   └── api/                     # 接口文档
├── scripts/
│   └── check_no_underscore_prefix.py  # 命名规范检查脚本
├── tests/
│   ├── conftest.py
│   ├── test_naming_convention.py
│   └── test_text_files_api.py
├── .cline/
│   └── skills/
│       └── no-underscore-prefix/    # Cline 项目级 Skill：命名规范
└── storage/uploads/text-files/  # 文本文件存储目录（自动创建）
```

### 新增功能模块的步骤

1. 在 `app/modules/` 下新建目录，例如 `images/`。
2. 在模块内编写 `constants.py` / `config.py` / `models.py` / `schemas.py` /
   `service.py` / `router.py`，模块自包含，不修改其它模块的代码。
3. 在 `app/router.py` 中注册模块的 router。
4. 在 `docs/api/routes/{模块名}/` 下补充接口文档（如 `text-files/`、`system/`），
   并在 `docs/api/README.md` 中登记。

这样每个功能都有独立的路由前缀（如 `/api/v1/text-files`、`/api/v1/images`），
既避免路由冲突，也让同类能力集中管理。

## 下一步

- [接口文档索引](./README.md) —— 查看完整接口列表与调用示例
- [配置说明](./configuration.md) —— 自定义上传目录、大小限制与类型白名单
- [错误码说明](./errors.md) —— 错误码定义与处理建议
