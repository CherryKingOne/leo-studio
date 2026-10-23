"""文本文件功能模块。

对外提供文本文件的上传、列表、详情、下载与删除能力。
路由前缀：``/api/v1/text-files``

模块内部文件说明：

- ``constants.py``：模块常量、扩展名白名单与 MIME 映射
- ``config.py``：模块独立配置
- ``models.py``：领域模型与分类规则
- ``schemas.py``：请求与响应 Schema
- ``parsers.py``：小说书名、章节切分与字数统计解析器
- ``csv_store.py``：CSV 数据表格管理与持久化驱动
- ``service.py``：文件存储业务逻辑
- ``dependencies.py``：依赖注入提供者
- ``router.py``：HTTP 路由定义
"""
