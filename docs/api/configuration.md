# 配置说明

Leo Studio 后端服务的全部配置项均可通过**环境变量**或项目根目录的 **`.env` 文件**覆盖。

## 配置分层

项目采用功能优先的模块组织方式，配置同样分层：

| 层级 | 文件 | 环境变量前缀 | 作用范围 |
| --- | --- | --- | --- |
| 全局 | `app/shared/config.py` | `LEO_` | 应用级通用配置 |
| 功能模块 | `app/modules/*/config.py` | `LEO_<模块>_` | 单个功能模块的专属配置 |

模块级配置**优先级高于**全局配置。例如文本文件模块设置了
`LEO_TEXT_FILES_MAX_UPLOAD_SIZE_MB` 时，会覆盖全局的 `LEO_MAX_UPLOAD_SIZE_MB`。

## 全局配置

| 配置项 | 环境变量 | 类型 | 默认值 | 说明 |
| --- | --- | --- | --- | --- |
| `app_name` | `LEO_APP_NAME` | `string` | `Leo Studio API` | 服务名称 |
| `app_version` | `LEO_APP_VERSION` | `string` | `0.1.0` | 服务版本号 |
| `debug` | `LEO_DEBUG` | `boolean` | `true` | 调试模式 |
| `cors_origins` | `LEO_CORS_ORIGINS` | `list[str]` | `["*"]` | 允许跨域的来源，逗号分隔 |
| `max_upload_size_mb` | `LEO_MAX_UPLOAD_SIZE_MB` | `integer` | `50` | 全局上传大小上限（MB） |

## 文本文件模块配置

配置前缀 `LEO_TEXT_FILES_`：

| 配置项 | 环境变量 | 类型 | 默认值 | 说明 |
| --- | --- | --- | --- | --- |
| `allowed_extensions` | `LEO_TEXT_FILES_ALLOWED_EXTENSIONS` | `list[str]` | 见下表 | 允许上传的扩展名白名单，逗号分隔；留空表示不限制 |
| `max_upload_size_mb` | `LEO_TEXT_FILES_MAX_UPLOAD_SIZE_MB` | `integer \| null` | `null`（沿用全局） | 模块级大小上限（MB） |
| `upload_dir` | `LEO_TEXT_FILES_UPLOAD_DIR` | `path` | `storage/uploads/text-files` | 存储目录，相对路径基于项目根目录解析 |
| `max_filename_length` | `LEO_TEXT_FILES_MAX_FILENAME_LENGTH` | `integer` | `255` | 文件名最大长度 |

默认允许的扩展名：

| 分类 | 扩展名 |
| --- | --- |
| 纯文本与文档 | `.txt` `.md` `.markdown` `.rst` `.log` |
| 结构化数据 | `.csv` `.tsv` `.json` `.yaml` `.yml` `.xml` |
| 配置文件 | `.toml` `.ini` `.cfg` `.conf` `.env` |
| 源码 | `.py` `.js` `.jsx` `.ts` `.tsx` `.css` `.html` `.htm` `.sql` |

## 使用示例

在项目根目录创建 `.env` 文件：

```dotenv
# 全局配置
LEO_APP_NAME=Leo Studio API
LEO_DEBUG=false
LEO_CORS_ORIGINS=http://localhost:5173
LEO_MAX_UPLOAD_SIZE_MB=100

# 文本文件模块配置
LEO_TEXT_FILES_ALLOWED_EXTENSIONS=.txt,.md,.csv,.json,.py
LEO_TEXT_FILES_MAX_UPLOAD_SIZE_MB=20
LEO_TEXT_FILES_UPLOAD_DIR=storage/uploads/text-files
```

## 新模块的配置约定

新增功能模块时，在模块目录下创建 `config.py`：

```python
from pydantic_settings import BaseSettings, SettingsConfigDict


class MyModuleSettings(BaseSettings):
    model_config = SettingsConfigDict(
        env_prefix="LEO_MY_MODULE_",  # 模块专属前缀
        env_file=".env",
        extra="ignore",
    )

    some_option: str = "default"
```

模块配置与全局配置一样使用 `lru_cache` 缓存单例，
通过 `app/modules/*/dependencies.py` 暴露为 FastAPI 依赖。

## 相关文档

- [快速开始](./getting-started.md)
- [错误码说明](./errors.md)
