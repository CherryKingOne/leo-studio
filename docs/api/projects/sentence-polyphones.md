# 句子多音字标注管理

管理指定句子下的多音字注音数据，支持查询、全量覆盖保存与清空标注。

| 属性 | 值 |
| --- | --- |
| **所属模块** | 剧目与项目管理（`app/modules/projects/`） |
| **持久化表** | `.leo-studio/novel_polyphones.csv`（小说多音字表，对应 小说_多音字.csv） |
| **关联方式** | 通过 `sentence_id` 关联至 `novel_sentences.csv` |
| **响应格式** | `application/json` |

---

## 1. 查询句子多音字标注列表

### 接口地址
`GET /api/v1/projects/{project_id}/sentences/{sentence_id}/polyphones`

### 路径参数
| 名称 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `project_id` | `string` | **是** | 项目唯一标识 (UUID4) |
| `sentence_id` | `string` | **是** | 句子唯一标识 (UUID4) |

### 响应字段 (`SentencePolyphoneListResponse`)
| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `sentence_id` | `string` | 所属句子 ID (UUID4) |
| `total` | `integer` | 标注总数 |
| `items` | `SentencePolyphoneItem[]` | 多音字标注列表（按 `char_index` 升序排列） |

#### `SentencePolyphoneItem` 元素说明
| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `polyphone_id` | `string` | 多音字标注唯一标识 (UUID4，32 位十六进制) |
| `sentence_id` | `string` | 所属句子 ID (UUID4) |
| `char_index` | `integer` | 多音字在句子中的字符偏移量（从 0 开始） |
| `character` | `string` | 被标注的汉字单字 |
| `pinyin` | `string` | 用户指定的拼音（如 `xíng` 或 `xing2`） |
| `created_at` | `string` | 创建时间 (ISO 8601) |
| `updated_at` | `string` | 最后更新时间 (ISO 8601) |

---

## 2. 全量覆盖保存句子多音字标注

### 接口地址
`PUT /api/v1/projects/{project_id}/sentences/{sentence_id}/polyphones`

### 请求体 (`SentencePolyphoneSaveRequest`)
```json
{
  "annotations": [
    {
      "char_index": 1,
      "character": "行",
      "pinyin": "xíng"
    },
    {
      "char_index": 6,
      "character": "行",
      "pinyin": "háng"
    }
  ]
}
```

### 校验规则
1. `project_id` 和 `sentence_id` 必须存在且具备归属关系。
2. `char_index` 必须在句子长度范围内 `[0, len - 1]`。
3. `character` 必须与句子在 `char_index` 处的实际汉字完全一致。
4. 单个请求内 `char_index` 不得重复。
5. `pinyin` 不能为空。

---

## 3. 清空句子多音字标注

### 接口地址
`DELETE /api/v1/projects/{project_id}/sentences/{sentence_id}/polyphones`

### 响应字段 (`SentencePolyphoneDeleteResponse`)
| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `sentence_id` | `string` | 所属句子 ID (UUID4) |
| `deleted_count` | `integer` | 被删除的标注数量 |
| `deleted` | `boolean` | 是否删除成功 (`true`) |
