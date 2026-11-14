# 小说台词切分（引号对白分离）开发规划

> 状态：**P0 已实现**（后端切分、独立 CSV 落盘、删除/覆盖/追更生命周期、测试）
>
> 目标：在现有「章节 → 行 → 句子」切句之后，新增一层「台词二次切分」。当一行正文中出现中文引号对白时，把该行拆成「叙述部分 + 对白部分」，并将拆分结果单独写入新的 CSV，**不改动现有 `novel_sentences.csv`**。

---

## 1. 结论摘要

| 项目 | 推荐方案 |
| --- | --- |
| 新 CSV 文件名 | **`novel_dialogues.csv`** |
| CSV 表名 / 常量名 | `novel_dialogues` / `NOVEL_DIALOGUES_CSV_FILENAME` |
| 存储位置 | `.leo-studio/novel_dialogues.csv`（与现有小说 CSV 同目录） |
| 切分入口 | `app/modules/text_files/processor.py` 的切句之后、`build_sentence_records` 附近 |
| 与切句表的关系 | `novel_dialogues.sentence_id` → `novel_sentences.sentence_id`，一对多 |
| 一期范围 | 只做后端切分 + 新 CSV 落盘；不改 `novel_sentences.csv`、不改 API、不改前端 |
| 二期范围（可选） | 历史数据回填、查询接口、工作台/选角消费 |

**核心原则：**

1. `novel_sentences.csv` 仍是唯一稳定的「行级切句」事实表，结构和内容不变。
2. 新表只保存「被引号切分出来的片段」，不是整本书所有句子的副本。
3. 多音字标注、前端工作台、现有接口均不受影响。
4. 先离线/服务端生成，后续再决定是否给前端消费。

---

## 2. 现状分析

### 2.1 当前数据处理链路

```
上传 .txt
  ↓
TextFileStorageService.save_file()          app/modules/text_files/service.py:165
  ↓
clean_text()                                 app/modules/text_files/parsers.py:701
  ↓
parse_chapters()                             app/modules/text_files/parsers.py:724
  ↓
每章：processor.create_sentence_records()    app/modules/text_files/processor.py:86
  ↓
split_sentences_by_lines()                   app/modules/text_files/processor.py:19
  ↓
build_sentence_records()                     app/modules/text_files/processor.py:40
  ↓
novel_sentences.csv                          app/modules/text_files/service.py:283-285
```

### 2.2 关键文件与职责

| 文件 | 当前职责 | 本次是否改动 |
| --- | --- | --- |
| `app/modules/text_files/parsers.py` | 清洗、分章、字数统计 | 否 |
| `app/modules/text_files/processor.py` | 按行切句、生成切句记录 | **是，新增台词切分函数/编排** |
| `app/modules/text_files/constants.py` | 文件名、表头常量 | **是，新增 `novel_dialogues` 常量** |
| `app/modules/text_files/config.py` | 模块配置 | **是，新增文件名配置项** |
| `app/modules/text_files/models.py` | 领域模型 | **是，新增 `DialogueSegmentInfo`** |
| `app/modules/text_files/service.py` | 落盘、查询、删除 | **是，新增表与写入/删除逻辑** |
| `app/modules/text_files/csv_store.py` | CSV 表格驱动 | 否，直接复用 `CsvTable` |
| `app/modules/projects/service.py` | 追更、覆盖章节 | **是，同步维护新表** |
| `tests/` | 测试 | **是，新增切分单测与集成测试** |

### 2.3 为什么必须单独建表

- `novel_sentences.csv` 的 `sentence_index` 是章节内行号，前端工作台、接口、测试都依赖它；往里插入「半句」会改变行号语义。
- 多音字标注通过 `sentence_id` 关联，不能因为台词切分而改变句子 ID 或句子内容。
- 台词切分是「可再解释的派生数据」，适合独立存储、独立重跑、独立演进。
- 单独表可以只保存真正被切分的行，避免给整本书所有句子复制一份。

---

## 3. 切分规则

### 3.1 基本规则

设一行正文为 `text`，且 `text` 已经是 `clean_line_text()` 清洗后的内容。

**触发条件：**

1. 行内存在一个**完整的中文引号对**：`“ ”`。
2. 开引号左侧有**非空叙述内容**（去掉首尾空白后不为空）。
3. 开引号右侧能找到同类型的闭引号。

**切分结果：**

- 第 1 段：`narration`，开引号左侧的叙述文本（去掉首尾空白）。
- 第 2 段：`dialogue`，从开引号开始、到闭引号结束（含引号）的对白文本。
- 闭引号之后如果还有内容，一期**并入第 2 段**，保证「一行最多两段」且全文不丢字。
- 开引号左侧为空（整行就是 `“……”`）时，一期**不拆分**，不写入新表。
- 没有闭引号时，**不拆分**，按普通句子处理。
- 一行出现多对引号时，一期只切**第一对**，其余内容留在第 2 段。

### 3.2 示例

输入：

```text
唐三对着比比东道：“复活吧我的爱人”
```

切句后 `novel_sentences.csv` 仍为 1 行：

| sentence_id | sentence_index | content |
| --- | --- | --- |
| `s1` | `0` | `唐三对着比比东道：“复活吧我的爱人”` |

`novel_dialogues.csv` 新增 2 行：

| dialogue_id | sentence_id | segment_index | segment_type | content | char_start | char_end | quote_char |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `d1` | `s1` | `0` | `narration` | `唐三对着比比东道：` | `0` | `9` | `""` |
| `d2` | `s1` | `1` | `dialogue` | `“复活吧我的爱人”` | `9` | `18` | `“` |

> `char_start` / `char_end` 按 Unicode 码点计算，左闭右开；与多音字 `char_index` 的口径一致。

### 3.3 边界用例

| 输入 | 结果 | 说明 |
| --- | --- | --- |
| `唐三对着比比东道：“复活吧我的爱人”` | 拆 2 段 | 标准场景 |
| `“复活吧我的爱人”` | 不写入新表 | 整行纯对白，无叙述前缀 |
| `唐三对着比比东道：复活吧我的爱人` | 不写入新表 | 没有引号 |
| `唐三对着比比东道：“复活吧我的爱人` | 不写入新表 | 引号未闭合，暂不处理 |
| `唐三对着比比东道：“复活吧我的爱人”然后走了。` | 拆 2 段，第 2 段为 `“复活吧我的爱人”然后走了。` | 一期尾部叙述并入对白段 |
| `他说：“第一句”，然后说：“第二句”` | 拆 2 段，第 2 段为 `“第一句”，然后说：“第二句”` | 一期只切第一对 |
| `他说：“”` | 拆 2 段，对白段为 `“”` | 空对白是否保留，见待确认项 |
| `他说：` | 不写入新表 | 无引号 |
| `“你好”唐三笑道。` | 不写入新表 | 前缀为空，一期不切 |

### 3.4 伪代码

```python
OPEN_TO_CLOSE = {"“": "”"}

def split_dialogue_line(text: str) -> list[DialogueSegment]:
    text = text.strip()
    if not text:
        return []

    for index, char in enumerate(text):
        close_char = OPEN_TO_CLOSE.get(char)
        if close_char is None:
            continue

        close_index = text.find(close_char, index + 1)
        if close_index == -1:
            continue

        prefix = text[:index].strip()
        if not prefix:
            return []

        return [
            DialogueSegment(
                segment_index=0,
                segment_type="narration",
                content=prefix,
                char_start=0,
                char_end=index,
                quote_char="",
            ),
            DialogueSegment(
                segment_index=1,
                segment_type="dialogue",
                content=text[index:],
                char_start=index,
                char_end=len(text),
                quote_char=char,
            ),
        ]

    return []
```

> 一期只支持 `“ ”`。`「 」`、`『 』`、ASCII `"` 是否纳入，见待确认项。

---

## 4. 新 CSV 设计

### 4.1 文件名

**推荐：`novel_dialogues.csv`**

命名理由：

- 与现有 `novels.csv`、`novel_contents.csv`、`novel_sentences.csv`、`novel_polyphones.csv` 保持 `novel_` 前缀。
- `dialogues` 是业务语义（台词/对白），比 `quotes`、`lines` 更准确。
- 与 `novel_sentences.csv` 形成清晰的「句子 → 台词片段」层级。

备选名对比：

| 文件名 | 评价 |
| --- | --- |
| **`novel_dialogues.csv`** | **推荐**：简洁、与现有命名一致 |
| `novel_dialogue_segments.csv` | 语义最精确（强调「片段」），但偏长 |
| `novel_dialogue_splits.csv` | 强调「拆分结果」，但 `split` 更像一次性动作 |
| `novel_quotes.csv` | 只强调引号，覆盖不了后续无引号对白 |
| `novel_speech.csv` | TTS 语义强，但范围比「台词」更宽 |
| `chapter_dialogues.csv` | 与其他 `novel_` 前缀表不一致 |

### 4.2 表头字段

```text
dialogue_id,novel_id,chapter_id,sentence_id,sentence_index,segment_index,segment_type,content,word_count,char_start,char_end,quote_char,created_at
```

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `dialogue_id` | string | 片段唯一标识，`uuid4().hex` |
| `novel_id` | string | 所属小说/文件 ID，对应 `novels.novel_id` |
| `chapter_id` | string | 所属章节 ID，对应 `novel_contents.chapter_id` |
| `sentence_id` | string | 来源句子 ID，对应 `novel_sentences.sentence_id` |
| `sentence_index` | int | 来源句在章节内的行索引，冗余保存便于查询 |
| `segment_index` | int | 同一来源句内的片段序号，从 0 开始 |
| `segment_type` | string | `narration` / `dialogue` |
| `content` | string | 片段正文 |
| `word_count` | int | 非空白字符数，复用 `count_words()` |
| `char_start` | int | 在来源句中的起始码点偏移，左闭 |
| `char_end` | int | 在来源句中的结束码点偏移，右闭开区间 |
| `quote_char` | string | 对白段的开引号字符；叙述段为空 |
| `created_at` | string | ISO 8601 时间 |

### 4.3 约束与索引

CSV 没有数据库索引，建议在应用层保证：

- `(sentence_id, segment_index)` 唯一。
- 同一 `sentence_id` 的片段按 `segment_index` 升序排列。
- 同一 `chapter_id` 的片段在重写章节时整批删除后重建。
- 查询按 `novel_id` / `chapter_id` / `sentence_id` 过滤；数据量大时全表扫描的成本见第 9 节。

### 4.4 与现有表的关系

```
novels (novel_id)
  └── novel_contents (chapter_id, novel_id)
        └── novel_sentences (sentence_id, chapter_id, novel_id)
              └── novel_dialogues (dialogue_id, sentence_id, chapter_id, novel_id)
```

- 一个句子最多产生 2 个片段（一期）。
- 未被切分的句子在新表中没有记录。
- 删除小说时按 `novel_id` 级联删除。
- 覆盖重写章节时按 `chapter_id` 删除旧片段再写入新片段。

---

## 5. 处理流程设计

### 5.1 新增模块/函数

建议新增 `app/modules/text_files/dialogue.py`，只放纯函数，不依赖 FastAPI/文件系统：

```python
@dataclass(slots=True)
class DialogueSegment:
    segment_index: int
    segment_type: str
    content: str
    char_start: int
    char_end: int
    quote_char: str

def split_dialogue_line(text: str) -> list[DialogueSegment]: ...
def build_dialogue_records(
    novel_id: str,
    chapter_id: str,
    sentence_records: list[dict[str, str]],
) -> list[dict[str, str]]: ...
```

`build_dialogue_records` 的输入直接使用 `build_sentence_records()` 的返回值，避免再次查询 CSV：

```python
for sentence in sentence_records:
    for segment in split_dialogue_line(sentence["content"]):
        records.append({
            "dialogue_id": uuid.uuid4().hex,
            "novel_id": novel_id,
            "chapter_id": chapter_id,
            "sentence_id": sentence["sentence_id"],
            "sentence_index": sentence["sentence_index"],
            "segment_index": str(segment.segment_index),
            "segment_type": segment.segment_type,
            "content": segment.content,
            "word_count": str(count_words(segment.content)),
            "char_start": str(segment.char_start),
            "char_end": str(segment.char_end),
            "quote_char": segment.quote_char,
            "created_at": now_iso,
        })
```

### 5.2 上传/首次保存

在 `TextFileStorageService.save_file()` 中：

1. 现有逻辑生成 `chapter_records` 与 `sentence_records`。
2. 新增：基于同一批 `sentence_records` 调用 `build_dialogue_records()`。
3. 批量写入 `novel_dialogues.csv`。

伪代码：

```python
sentence_records = []
dialogue_records = []

for idx, (ch_title, ch_content) in enumerate(chapters):
    ch_id = uuid.uuid4().hex
    chapter_records.append({...})

    if ch_content:
        chapter_sentences = self.processor.create_sentence_records(
            novel_id=file_id,
            chapter_id=ch_id,
            content=ch_content,
        )
        sentence_records.extend(chapter_sentences)
        dialogue_records.extend(
            build_dialogue_records(file_id, ch_id, chapter_sentences)
        )

self.contents_table.insert_batch(chapter_records)
if sentence_records:
    self.sentences_table.insert_batch(sentence_records)
if dialogue_records:
    self.dialogues_table.insert_batch(dialogue_records)
```

### 5.3 追更/增量追加

在 `ProjectService.append_chapters()` 中：

- **新增章节**：与首次上传一致，生成 `sentence_records` 的同时生成 `dialogue_records`，最后批量写入。
- **覆盖已有章节**：
  1. 删除该章节旧的 `novel_dialogues` 记录：`dialogues_table.delete_by("chapter_id", old_ch_id)`。
  2. 重写 `novel_sentences` 后，基于新切句生成新的 `dialogue_records` 并写入。
  3. 注意：**按章节删一次**，不要在循环里逐句 `delete_by("sentence_id")`，否则每次删除都会重写整张 CSV。
- **跳过已有章节**：不动新表。
- **`dry_run=True`**：不写任何表，和现有行为一致。

### 5.4 删除与清理

- `TextFileStorageService.delete_file()`：新增 `dialogues_table.delete_by("novel_id", file_id)`。
- `TextFileStorageService.cleanup()`：新增 `dialogues_table.clear()`。
- `ProjectService.hard_delete_project()` / `clear_recycle_bin()`：复用 `storage.delete_file()`，自动级联。

### 5.5 配置与常量

`constants.py`：

```python
NOVEL_DIALOGUES_CSV_FILENAME: Final[str] = "novel_dialogues.csv"

NOVEL_DIALOGUES_COLUMNS: Final[list[str]] = [
    "dialogue_id",
    "novel_id",
    "chapter_id",
    "sentence_id",
    "sentence_index",
    "segment_index",
    "segment_type",
    "content",
    "word_count",
    "char_start",
    "char_end",
    "quote_char",
    "created_at",
]
```

`config.py`：

```python
novel_dialogues_csv_filename: str = NOVEL_DIALOGUES_CSV_FILENAME
```

环境变量：`LEO_TEXT_FILES_NOVEL_DIALOGUES_CSV_FILENAME`。

`service.py`：

```python
self.dialogues_table = CsvTable(
    self.csv_dir / self.settings.novel_dialogues_csv_filename,
    columns=NOVEL_DIALOGUES_COLUMNS,
)
```

---

## 6. 对现有接口/前端的影响

### 6.1 一期：无影响

- `novel_sentences.csv` 结构、内容、`sentence_index` 不变。
- `GET /api/v1/text-files/{file_id}/sentences` 不变。
- `GET /api/v1/projects/{id}/chapters/{chapterId}` 不变。
- 多音字标注接口不变。
- 前端工作台仍按 `sentences[]` 渲染，不消费新表。

### 6.2 二期（可选）：查询接口

如果后续需要给选角/工作台消费，可新增：

```text
GET /api/v1/text-files/{file_id}/dialogues
GET /api/v1/text-files/{file_id}/dialogues?chapter_id=xxx
GET /api/v1/projects/{project_id}/chapters/{chapter_id}/dialogues
```

或者在 `ChapterContentResponse` 中**可选新增**：

```json
{
  "sentences": [...],
  "dialogues": [
    {
      "dialogue_id": "...",
      "sentence_id": "...",
      "segment_index": 0,
      "segment_type": "narration",
      "content": "唐三对着比比东道："
    }
  ]
}
```

注意：这会改动响应结构，需前端同步；一期不做。

---

## 7. 测试计划

### 7.1 纯函数单测（`tests/test_dialogue_split.py`）

| 用例 | 期望 |
| --- | --- |
| `唐三对着比比东道：“复活吧我的爱人”` | 2 段，类型/内容/偏移正确 |
| `“复活吧我的爱人”` | 0 段（一期不切） |
| `唐三对着比比东道：复活吧我的爱人` | 0 段 |
| `唐三对着比比东道：“复活吧我的爱人` | 0 段 |
| `他说：“第一句”，然后说：“第二句”` | 2 段，第 2 段保留第一对引号及其余内容 |
| `他说：“你好”然后走了。` | 2 段，尾部并入对白段 |
| `他说：“”` | 2 段（或按最终决策为 0 段） |
| 空字符串 / 纯空白 | 0 段 |
| `「你好」` | 一期 0 段（未启用） |

### 7.2 集成测试

- 上传包含标准示例的小说 → `novel_dialogues.csv` 表头正确、恰好 2 行、内容与偏移正确。
- `novel_sentences.csv` 仍为 1 行，内容为完整原句。
- 纯叙述行不产生新表记录。
- 删除小说 → 新表对应记录被级联删除。
- 追更覆盖已有章节 → 旧片段被删除、新片段写入，不残留重复。
- `dry_run` → 新表无变化。
- 现有 `tests/test_text_files_api.py`、`tests/test_projects_api.py`、`tests/test_sentence_polyphones_api.py` 全部通过。

### 7.3 验收标准

1. 示例输入产出且仅产出 2 条片段记录。
2. `novel_sentences.csv` 内容与行数不因新功能改变。
3. 新表文件名、表头、字段顺序与本文一致。
4. 删除/覆盖/追更三条生命周期均不残留脏数据。
5. 现有测试无回归。

---

## 8. 实施步骤（建议顺序）

- [ ] **P0：确认方案** — 确认文件名、存储粒度、边界规则。
- [ ] **P0：常量与配置** — `constants.py`、`config.py`、`models.py`。
- [ ] **P0：纯切分函数** — `dialogue.py` + 单测。
- [ ] **P0：存储接入** — `service.py` 新增表、保存、删除、清理。
- [ ] **P0：追更接入** — `projects/service.py` 新增/覆盖章节。
- [ ] **P0：集成测试** — 上传、删除、覆盖、`dry_run`。
- [ ] **P1：历史回填脚本** — `scripts/backfill_novel_dialogues.py`。
- [ ] **P1：查询接口** — 按需新增，不在本次范围。
- [ ] **P1：前端消费** — 选角/工作台按需接入。

---

## 9. 风险与权衡

| 风险 | 说明 | 缓解 |
| --- | --- | --- |
| 引号误判 | 正文中的引用、书名号、术语可能包含 `“”` | 一期按需求全量切分；后续可加白名单/上下文规则 |
| 多引号/嵌套引号 | 一行多对引号时只切第一对 | 一期明确限制；二期再扩展 |
| 尾部叙述 | 闭引号后的叙述并入对白段，语义不纯 | 二期增加 `narration_tail` 段或 `tail` 字段 |
| 历史数据 | 老项目没有 `novel_dialogues` 记录 | 一期不回填；二期提供幂等回填脚本 |
| CSV 性能 | 查询全表扫描；删除/更新整文件重写 | 数据量大后迁移到 SQLite/DB；按章节批量删除 |
| 跨表一致性 | 句子写入成功、台词写入失败会留下半数据 | 与现有 CSV 方案一致，先保持 best-effort；后续可加补偿脚本 |
| 纯对白行 | 整行 `“……”` 不进入新表 | 二期可选择写入一条 `dialogue` 记录 |

---

## 10. 待确认问题

1. **文件名**：确认使用 `novel_dialogues.csv` 吗？还是更倾向 `novel_dialogue_segments.csv`？
2. **存储粒度**：确认「被切分的行写 2 条片段」？还是只写对白段 1 条？
3. **纯对白行**：整行就是 `“……”` 时，是否也要写入新表？
4. **多引号**：一行多对引号，一期只切第一对，是否可以接受？
5. **尾部叙述**：闭引号后的叙述并入对白段，是否可以接受？还是必须拆成第 3 段？
6. **引号范围**：一期只支持 `“ ”`；是否需要同时支持 `「 」`、`『 』`、ASCII `"`？
7. **历史数据**：一期是否接受老项目没有台词数据？
8. **接口暴露**：一期是否完全不改 API/前端？

---

## 11. 推荐决策（供确认）

- 文件名：**`novel_dialogues.csv`**。
- 存储粒度：每个被切分的句子写 **2 条**片段（`narration` + `dialogue`）。
- 纯对白行：一期 **不写入**新表。
- 多引号：一期 **只切第一对**。
- 尾部叙述：一期 **并入对白段**。
- 引号范围：一期 **仅 `“ ”`**。
- 历史数据：一期 **不回填**。
- 接口/前端：一期 **不改**。

确认后即可按第 8 节开始实现。

---

## 12. 实现记录（P0）

已完成：

- `app/modules/text_files/dialogue.py`：`split_dialogue_line()`、`build_dialogue_records()`。
- `app/modules/text_files/models.py`：`DialogueSegmentType`、`DialogueSegment`。
- `app/modules/text_files/constants.py`：`NOVEL_DIALOGUES_CSV_FILENAME`、`NOVEL_DIALOGUES_COLUMNS`。
- `app/modules/text_files/config.py`：`novel_dialogues_csv_filename`。
- `app/modules/text_files/service.py`：新增 `dialogues_table`；上传写入、删除级联、`cleanup`、章节/全书查询。
- `app/modules/projects/service.py`：增量追更写入、覆盖章节时删除旧记录并重建。
- `tests/test_dialogue_split.py`：切分规则、记录构造、上传落盘、级联删除、追更/覆盖生命周期。

未做（按一期范围）：

- 历史数据回填脚本。
- 台词查询 API。
- 前端工作台/选角消费。
- `「 」`、`『 』`、ASCII 引号支持。

验证命令（本容器未安装 Python，未执行）：

```bash
uv run pytest tests/test_dialogue_split.py
uv run pytest
```