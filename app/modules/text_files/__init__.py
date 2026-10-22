"""文本文件功能模块。

对外提供文本文件的上传、列表、详情、下载与删除能力。
路由前缀：``/api/v1/text-files``

模块内部文件说明：

- ``constants.py``：模块常量与文本文件类型白名单
- ``config.py``：模块独立配置
- ``models.py``：领域模型与分类规则
- ``schemas.py``：请求与响应 Schema
- ``service.py``：文件存储业务逻辑
- ``dependencies.py``：依赖注入提供者
- ``router.py``：HTTP 路由定义
"""
