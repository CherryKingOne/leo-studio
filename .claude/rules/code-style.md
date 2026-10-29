# Leo Studio 代码风格规则

## 命名规范：禁止下划线开头的函数/方法/类名

**规则级别：强制（MUST NOT）**

项目内所有 Python 代码中，**禁止出现以下划线 `_` 开头的函数、方法或类定义**。

### 具体禁止的命名

以下写法在项目中属于**违规**：

```python
# fmt: off
# ❌ 模块级函数
def _helper():
    ...

# ❌ 异步函数
async def _helper():
    ...

# ❌ 类
class _Helper:
    ...

# ❌ 类方法
class Service:
    def _method(self):
        ...

# ❌ 静态方法
class Service:
    @staticmethod
    def _method():
        ...

# ❌ 类方法（classmethod）
class Service:
    @classmethod
    def _method(cls):
        ...
# fmt: on
```

同时，**禁止调用以下划线开头的方法**，因为项目中不应存在此类定义：

```python
# fmt: off
_text.str()          # ❌ _text 不应存在
some_obj._internal() # ❌ _internal 不应存在
# fmt: on
```

### 允许的命名

以下写法是**允许**的：

```python
# fmt: off
# ✅ 普通函数
def helper():
    ...

# ✅ 异步函数
async def helper():
    ...

# ✅ 类
class Helper:
    ...

# ✅ 类方法
class Service:
    def method(self):
        ...

# ✅ 双下划线魔术方法（Python 协议保留名，予以豁免）
class Service:
    def __init__(self): ...
    def __repr__(self): ...
    def __str__(self): ...
    def __enter__(self): ...
    def __exit__(self, *args): ...
# fmt: on
```

### 豁免条件

唯一的豁免是**双下划线包裹的魔术方法**（`__init__`、`__repr__`、`__str__`、`__enter__`、`__exit__` 等），因为它们是 Python 语言协议规定的保留名称，必须保持原有拼写。

### 适用范围

| 范围 | 是否强制 |
| --- | --- |
| `app/` 下的所有 Python 文件 | ✅ 是 |
| `tests/` 下的所有 Python 文件 | ✅ 是 |
| `scripts/` 下的所有 Python 文件 | ✅ 是 |
| 变量名、私有属性（`self._cache`） | ❌ 不在强制范围 |

### 检查方式

运行以下命令检查项目是否符合规则：

```bash
python scripts/check_no_underscore_prefix.py
```

或通过测试：

```bash
python -m pytest tests/test_naming_convention.py
```

### 修复方式

如果发现违规命名，应将定义及其所有调用处的下划线前缀去掉：

| 违规写法 | 修复写法 |
| --- | --- |
| `def _helper():` | `def helper():` |
| `def _method(self):` | `def method(self):` |
| `class _Helper:` | `class Helper:` |

---

## 目录结构规范：按功能模块化拆分

**规则级别：强制（MUST）**

项目内所有新功能必须**以功能（业务领域）为边界拆分为独立模块**，放置在 `app/modules/` 目录下，不要将多个功能的代码混合在一起。

### 核心原则

1. **一个功能 = 一个模块目录**，每个模块内部包含该功能所需的全部文件（模型、路由、服务、配置、Schema 等）
2. **模块之间互不感知**，跨模块通信通过 `app/router.py` 聚合，或引入显式依赖
3. **新增功能时新建模块**，不要往已有模块中塞入不相关的逻辑

### 模块目录结构（以 `text_files` 为参考范例）

```
app/modules/text_files/
├── __init__.py       ← 模块说明文档：功能概述、路由前缀、内部文件清单
├── constants.py      ← 模块常量：白名单、映射表、Final 标记的不可变值
├── config.py         ← 模块独立配置：Pydantic BaseSettings，带 env_prefix
├── schemas.py        ← 请求/响应 Schema：Pydantic BaseModel，仅描述接口契约
├── models.py         ← 领域模型：dataclass / StrEnum，不依赖任何 Web 框架
├── service.py        ← 业务逻辑层：文件存储、校验、索引读写等核心算法
├── dependencies.py   ← 依赖注入：暴露 FastAPI Depends 目标，测试时可替换
└── router.py         ← HTTP 路由：路由定义与请求分发，不含业务逻辑
```

各文件职责说明：

| 文件 | 职责 | 关键依赖 |
| --- | --- | --- |
| `__init__.py` | 模块文档字符串，说明功能与内部文件 | 无 |
| `constants.py` | 模块内所有硬编码常量，使用 `Final` 标记 | 仅标准库 |
| `config.py` | 模块独立运行时配置，隔离于全局配置 | `pydantic-settings` |
| `schemas.py` | 输入/输出接口契约，与领域模型解耦 | `pydantic` |
| `models.py` | 模块内部数据结构与枚举，框架无关 | 标准库 |
| `service.py` | 核心业务逻辑，可被 router 和测试复用 | 模块内其他文件 |
| `dependencies.py` | 服务实例化的依赖注入提供者 | 模块内 service |
| `router.py` | FastAPI 路由，只做参数校验与响应格式化 | 模块内 schemas/dependencies |

### 路由注册方式

新功能模块创建后，只需在 `app/router.py` 中注册其路由：

```python
from app.modules.xxx.router import router as xxx_router
api_router.include_router(xxx_router)
```

### 新增功能模块示例

如需新增「图片文件」模块，应创建：

```
app/modules/image_files/
├── __init__.py
├── constants.py
├── config.py
├── schemas.py
├── models.py
├── service.py
├── dependencies.py
└── router.py
```

然后把 `image_files_router` 注册到 `app/router.py`，**不要**往 `text_files` 模块中写入图片相关的代码。

### 例外处理

如果两个功能确实需要共享逻辑：
- 将共享代码提取到 `app/shared/` 目录
- 不要直接在模块之间交叉引用对方的内部文件

---

## 导入规范：禁止相对导入，必须使用绝对导入

**规则级别：强制（MUST NOT）**

项目内所有 Python 后端代码中，**禁止任何形式的相对导入**，即模块名中出现 `.` / `..` 前缀的写法。所有导包必须使用**绝对导入**——从项目根即可完整解析的模块路径。

### 具体禁止的写法

以下写法在项目中属于**违规**：

```python
# fmt: off
# ❌ 单点相对导入（当前包）
from . import router
from .service import TextFileStorageService

# ❌ 多点相对导入（上级包）
from ..shared.response import success
from ..config import settings

# ❌ 更深层级相对导入
from ...modules.projects.models import Project

# ❌ 只引用上级包本身
from .. import shared
# fmt: on
```

### 正确写法

以下写法是**推荐 / 允许**的：

```python
# fmt: off
# ✅ 以 app. 起始的绝对导入
from app.modules.text_files.service import TextFileStorageService
from app.shared.response import success
from app.shared.config import settings

# ✅ 绝对导入 + 别名
from app.modules.projects import router as projects_router

# ✅ 第三方库与标准库导入
from fastapi import APIRouter
from pathlib import Path

# ✅ from __future__ import annotations 不属于相对导入
from __future__ import annotations

# ✅ tests/ 中通过 sys.path 引入 scripts 顶层模块（绝对模块名，无 . 前缀）
from check_no_underscore_prefix import scan
# fmt: on
```

### 为什么强制绝对导入

1. **避免同一模块被双重加载**：相对导入会让 `app.modules.x.service` 与 `modules.x.service`（或 `x.service`）以不同模块名分别驻留在 `sys.modules` 中，导致单例失效、`isinstance` 判定失败、FastAPI 依赖注入与 Pydantic 模型引用错乱。
2. **保证导入可追溯**：绝对路径直接表明「代码属于哪个模块」，跨目录移动文件时不会出现难以排查的相对层级失效。
3. **兼容多种启动方式**：`uvicorn app.main:app`、`python -m app.main`、`pytest` 等入口的工作目录与包根不同，只有绝对导入能稳定生效。
4. **与目录结构规范一致**：见上文「按功能模块化拆分」，模块间引用必须显式写明完整路径，才能保持模块边界清晰。

### 适用范围

| 范围 | 是否强制 |
| --- | --- |
| `app/` 下的所有 Python 文件（含 `__init__.py`、`router.py` 等） | ✅ 是 |
| `tests/` 下的所有 Python 文件 | ✅ 是 |
| `scripts/` 下的所有 Python 文件 | ✅ 是 |
| `desktop/`（Next.js / TypeScript 代码） | ❌ 不适用本规则 |
| 文件系统路径拼接（如 `Path(__file__).resolve().parent.parent`） | ❌ 不属于导包，不在本规则约束范围 |

### 无豁免

本规则**不设豁免条件**。`from __future__ import annotations`、标准库与第三方库导入本身不是相对导入，不受限制；只要 `import` / `from` 的子句中出现以 `.` 起始的模块路径即视为违规。

### 检查方式

使用以下命令扫描违规的相对导入（无输出即为符合规范）：

```bash
grep -rnE '^[[:space:]]*(from|import)[[:space:]]+\.' app tests scripts --include='*.py'
```

无输出即表示项目内不存在相对导入；每一行输出都需要按下方「修复方式」改为绝对导入。

### 修复方式

将相对模块路径补全为从项目根开始的绝对路径：

| 违规写法 | 修复写法 |
| --- | --- |
| `from . import service` | `from app.modules.xxx import service` |
| `from .service import Service` | `from app.modules.xxx.service import Service` |
| `from ..shared.response import success` | `from app.shared.response import success` |
| `from ..config import settings` | `from app.shared.config import settings` |
| `from ...modules.projects.models import Project` | `from app.modules.projects.models import Project` |

修改时以**当前文件在 `app/` 下的真实目录层级**推导完整路径，不要凭包内层级猜测；若发现需要跨模块引用内部文件，优先改为引用 `app/shared/` 中的公共代码。

---

## 规则维护方式

| 规则 | 自动化检查 | 备注 |
| --- | --- | --- |
| 禁止下划线开头的命名 | `scripts/check_no_underscore_prefix.py`、`tests/test_naming_convention.py` | 已接入测试 |
| 目录结构按功能模块化拆分 | 无 | 代码评审时人工核查 |
| 禁止相对导入 | 无 | 使用本文「检查方式」中的 grep 命令核查；如需长期固化，可在 `scripts/` 下新增扫描脚本并配套测试 |

其中「禁止下划线开头的命名」由 `.cline/skills/no-underscore-prefix/SKILL.md` 与 `scripts/check_no_underscore_prefix.py` 共同维护。
