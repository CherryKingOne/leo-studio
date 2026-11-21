# 注册层（Registry Layer）设计

> 分支：`feature/plugin-registry`（基于 `feature/plugin-framework`）。
> 目标：以后新增插件 **不改内核、不改接口层**，只“往注册表里注册”。

## 1. 参考：leo-agents / leo-plugin 是怎么做的

leo-agents 的内核只做三件事：服务注册（`ctx.provide` / `inject`）、事件、可撤销副作用（插件卸载时自动撤销它做过的注册）。
真正的“注册层”在 leo-plugin 里，是插件自己：

| 角色 | leo-plugin 里的例子 | 做什么 |
| --- | --- | --- |
| 注册层（大箱子） | `gateway`（`src/channels/registry.ts`）、`provider`（`src/registry.ts`） | 提供一个服务（`gateway` / `providers`），里面有驱动表；读配置（`gateway.json` 的 accounts、`provider.json`）决定哪个账号 / 模型用哪个驱动 |
| 子插件（小盒子） | `gateway-qq`、`gateway-feishu`、`provider-deepseek`、`provider-qianfan` | `inject: ['gateway']`，在 `apply` 里 `ctx.effect(() => gateway.registerChannel(driver))`，拿到撤销函数，卸载时自动撤销 |

几条关键规则：

1. **按驱动 id 注册**：QQ 驱动 id 是 `qq`，配置里 `"channel": "qq"` 的账号就用它。
2. **内置 / 插件两个槽位**：网关自带一份内置 qq 驱动；`gateway-qq` 注册同 id 驱动时顶替内置那份，卸载后内置那份自动恢复。
3. **注册顺序任意**：账号先登记为 `waiting_driver`，驱动一来就启动；provider 注册层每次变化都整体重新同步。
4. **子插件之间不互相 import**：只通过服务协作（`gateway-qq` 按结构复制一份契约 `contract.ts`）。
5. **配置驱动行为**：换驱动、加账号改配置文件，不改代码。

## 2. leo-studio 原来的问题

- 服务容器按“类型”存，一个类型只能有一个实现：CSV 导出器和 JSON 导出器互相覆盖；以后加第二个 TTS 也会覆盖 Edge。
- 接口层绕过插件：导出接口写死 `CsvExporter` / `JsonExporter`，导入接口没有导入器时写死 `TxtImporter`。
- 插件之间直接依赖：`importer-txt` 直接依赖 kaomoji / novel-regex / quotes 三个文本插件。
- 装配写死在 `api/src/plugins.rs` 和 `cli/src/lib.rs` 两处，API 插件依赖了全部具体插件。
- `PluginManager` 用 HashMap：顺序随机、同 id 静默覆盖、失败不回滚、不能卸载；API / MCP 插件在 `apply_all` 之后才登记，`apply` 从未执行。

## 3. 新结构

```text
leo-studio/
├── core/                              微内核（新增插件不再改它）
│   └── src/
│       ├── ast/                       剧本 AST
│       ├── context/                   Context：服务 + 事件 + 插件作用域（卸载自动撤销）
│       ├── registry/                  通用注册表 Registry<T>：驱动 id、内置/插件槽位、默认驱动
│       ├── plugin/                    Plugin trait、manifest（depends）、PluginManager（有序/依赖/回滚/卸载）
│       ├── spi/                       扩展点契约：tts / storage / exchange / text
│       └── pipeline/
├── plugins/
│   ├── text/
│   │   ├── registry/                  leo-plugin-text      注册层：清洗链 + 分章器 + 台词切分器
│   │   ├── cleaner-kaomoji/           驱动 kaomoji
│   │   ├── parser-novel-regex/        驱动 novel-regex
│   │   └── dialogue-quotes/           驱动 quotes
│   ├── exchange/
│   │   ├── registry/                  leo-plugin-exchange  注册层：按扩展名选导入器、按格式选导出器
│   │   ├── importers/txt/             驱动 txt（只依赖 leo-plugin-text）
│   │   └── exporters/{json,csv}/      驱动 json / csv
│   ├── storage/
│   │   ├── registry/                  leo-plugin-storage   注册层：默认存储
│   │   └── csv/                       驱动 csv
│   ├── tts/
│   │   ├── registry/                  leo-plugin-tts       注册层：默认驱动 + “驱动id:声线”路由
│   │   ├── edge/                      驱动 edge（微软 Edge，免费）
│   │   ├── stepfun/                   驱动 stepfun（阶跃星辰，需 STEPFUN_API_KEY）
│   │   │   ├── src/lib.rs             插件入口 + 官方文档链接
│   │   │   ├── src/config.rs          参数（leo-plugins.json → settings）与校验
│   │   │   ├── src/client.rs          /v1/audio/speech、/v1/audio/system_voices
│   │   │   ├── src/voices.rs          内置兜底音色
│   │   │   ├── src/audio.rs           从 wav/mp3 算时长
│   │   │   └── tests/                 本地假服务器测试 + 真实调用测试（--ignored）
│   │   └── minimax/                   驱动 minimax（MiniMax，需 MINIMAX_API_KEY）
│   │       ├── src/lib.rs             插件入口 + 官方文档链接
│   │       ├── src/config.rs          参数与校验
│   │       ├── src/client.rs          /v1/t2a_v2（hex 音频）、/v1/get_voice
│   │       ├── src/voices.rs          兜底音色、情绪映射、音调换算
│   │       └── tests/
│   ├── interfaces/                    对外入口：只依赖 core + 各注册层
│   │   ├── api/
│   │   ├── mcp/
│   │   └── cli/
│   └── catalog/                       leo-plugin-catalog：编进程序的插件清单 + leo-plugins.json
├── app/                               宿主：catalog + API/MCP
└── leo-plugins.example.json           插件配置示例
```

依赖方向（箭头 = 依赖）：

```text
app / cli ──► catalog ──► 各注册层 + 各子插件 ──► core
api / mcp ──► 各注册层 ──► core
子插件     ──► core（必要时 + 别的注册层，如 importer-txt → leo-plugin-text）
```

子插件之间不再互相依赖；接口层不认识任何具体子插件。

## 4. 内核提供了什么

```rust
// 子插件：往注册表里注册驱动（插件卸载时自动撤销）
ctx.register::<dyn TTSProvider>("edge", Arc::new(EdgeTtsProvider::new()))?;

// 注册层：按配置设默认驱动，对外提供统一服务
let registry = ctx.registry::<dyn TTSProvider>();
registry.set_default(config.default.as_deref());
ctx.provide(TtsHub { registry }).await;

// 接口层：只问注册层
let tts = TtsHub::of(&ctx).await;
tts.synthesize(&payload).await?;          // "edge:zh-CN-XiaoxiaoNeural" 走 edge，没前缀走默认驱动
let (_, exporter) = ExchangeHub::of(&ctx).await.exporter(Some("csv"))?;
```

- 注册表在第一次访问时创建，所以注册层和子插件**谁先装配都行**（和 gateway 一样）。
- 同一 id 有内置和插件两份时插件那份生效，撤销后内置那份恢复。
- `TtsHub::of(ctx)` 等在注册层没装配时退回直接读注册表，不会因为少装一个注册层就整体不可用。

`PluginManager`：

- 登记顺序装配；`manifest.depends` 里的插件先装配；缺依赖 / 循环依赖报错。
- 同 id 重复登记报错。
- 任何一个插件装配失败：本次已装配的插件全部按相反顺序撤销。
- `unload(id)`：撤销该插件全部注册；依赖它的插件回到 Pending。
- `apply_all` 可重复调用，只装配新登记的插件。
- `statuses()`：每个插件的 pending / active / failed 与错误原因（`/api/v2/plugins`、`leo-cli plugins` 都会显示）。

## 5. 以后怎么加一个插件（以 CosyVoice TTS 为例）

1. 新建 `plugins/tts/cosyvoice/`（`leo-plugin-tts-cosyvoice`），只依赖 `leo-core`：
   ```rust
   async fn apply(&self, ctx: &Context) -> CoreResult<()> {
       ctx.register::<dyn TTSProvider>("cosyvoice", Arc::new(CosyVoiceProvider::new(..)))
   }
   ```
2. 根 `Cargo.toml` 的 members 加一行。
3. `plugins/catalog/Cargo.toml` 加依赖，`plugins/catalog/src/lib.rs` 的 `CATALOG` 加一项。
4. 想让它当默认：`leo-plugins.json` 里 `"tts": { "default": "cosyvoice" }`；不改默认也能用 `cosyvoice:声线id` 指定。

内核、TTS 注册层、API、MCP、CLI 都不用改。新增一种**全新的扩展点**（比如“多音字标注”）时：在 `core/src/spi/` 加契约，再新建一个 `plugins/<领域>/registry/` 注册层——这是唯一需要动内核的情况，而且只是加一个 trait。

## 6. 配置文件 `leo-plugins.json`

读取顺序：`LEO_PLUGINS_CONFIG` 环境变量指定的路径 > 当前目录的 `leo-plugins.json` > 默认配置。写错字段名会直接报错（`deny_unknown_fields`），不会静默忽略。

```json
{
  "disabled": ["leo-plugin-exporter-csv"],
  "text":     { "cleaners": ["kaomoji"], "chapter_splitter": "novel-regex", "dialogue_splitter": "quotes" },
  "exchange": { "default_exporter": "json" },
  "storage":  { "default": "csv" },
  "tts":      { "default": "edge" }
}
```

注册层插件不能停用（停用会报错）；子插件随便停。

`settings` 按插件 id 放各子插件自己的参数，由子插件自己解析校验，写错字段名或取值超范围启动时直接报错：

```json
"settings": {
  "leo-plugin-tts-stepfun": { "model": "step-tts-2", "response_format": "mp3", "default_voice": "cixingnansheng" },
  "leo-plugin-tts-minimax": { "model": "speech-2.8-hd", "format": "mp3", "default_voice": "male-qn-qingse" }
}
```

API Key 不写进配置文件，只从环境变量读（`STEPFUN_API_KEY`、`MINIMAX_API_KEY`）；没设置时插件照常装配、只是不注册驱动，日志里有警告。

## 7. 还没做的（与本次重构无关，单独排期）

- API 的 v2 项目流程仍用 `plugins/interfaces/api/src/text/` 里复制的文本算法和自己的 `Store`，没有走注册层（上一轮审查的“两套存储互相覆盖”问题）。注册层就绪后，下一步是把它迁到 `TextHub` / `StorageHub`。
- Rust 不做运行时动态加载（dylib / wasm），插件清单在编译期确定；启用 / 停用 / 选默认驱动在运行期由配置决定。

> 测试提示：stepfun / minimax 的测试会在 127.0.0.1 起一个假服务器。如果本机设置了 HTTP(S)_PROXY，请同时设置 `NO_PROXY=127.0.0.1,localhost`，否则请求会被代理转走导致 404。真实接口测试默认跳过，设置好 Key 后用 `cargo test -p leo-plugin-tts-stepfun -- --ignored`（会计费）。
