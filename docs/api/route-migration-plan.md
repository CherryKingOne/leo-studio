# API v3 项目管理迁移规划（现行范围）

> 状态：已完成
> 当前只保留 18 个 v3 项目管理接口；旧版本路由与兼容层已彻底移除。

## 1. 当前目标

API 只保留以下四个职责模块：

1. 项目管理
2. 项目章节管理
3. 项目多音字管理
4. 回收站管理

所有项目子资源路由必须携带完整的项目作用域：

- 章节：`project_id + chapter_id`
- 章节多音字：`project_id + chapter_id`
- 句子多音字：`project_id + chapter_id + sentence_id`

## 2. 目标路由

```text
/api/v3
├── projects
│   ├── /                          # 创建、列表
│   └── /{project_id}              # 详情、更新、软删除
├── projects/{project_id}/chapters
│   ├── /                          # 章节列表、批量追加
│   ├── /{chapter_id}              # 章节详情、删除
│   ├── /{chapter_id}/polyphones   # 章节多音字聚合查询
│   └── /{chapter_id}/sentences/{sentence_id}/polyphones
│       ├── /                      # 句子多音字查询、保存、清空
│       └── /items/{char_index}    # 删除单条标注
└── recycle-bin
    ├── /                          # 列表、清空
    ├── /{project_id}/restore      # 恢复
    └── /{project_id}              # 彻底删除
```

## 3. 模块边界

| 模块 | 职责 | 接口数 |
| --- | --- | --- |
| `projects` | 项目创建、查询、更新、软删除 | 5 |
| `chapters` | 项目章节列表、追加、详情、删除 | 4 |
| `polyphones` | 项目章节与句子多音字标注 | 5 |
| `recycle_bin` | 回收站列表、清空、恢复、彻底删除 | 4 |

模块可以独立拆分处理器和文档，但 URL 必须表达资源归属。

## 4. 已落地约束

- 只注册 `/api/v3` 路由。
- 不保留旧版本路由别名。
- 不保留重定向或双处理器。
- 多音字只有一套 canonical 路径。
- 项目章节与多音字必须校验完整 ID 链。
- 每个接口都有独立 Markdown 文档。
- 文档目录只保留当前四个模块。

## 5. 当前文档目录

```text
docs/api/
├── README.md
├── route-migration-plan.md
├── projects/
├── project-chapters/
├── project-polyphones/
└── recycle-bin/
```

## 6. 验收结果

- [x] API 只暴露 `/api/v3/**` 业务路由。
- [x] 项目管理 5 个接口通过测试。
- [x] 项目章节管理 4 个接口通过测试。
- [x] 项目多音字管理 5 个接口通过测试。
- [x] 回收站管理 4 个接口通过测试。
- [x] 旧版本路径返回 404。
- [x] 18 个接口均有独立 Markdown 文档。
- [x] 代码中不存在旧版本兼容分支、别名或重定向。
