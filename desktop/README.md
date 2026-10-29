# VoiceNovel Studio · desktop

Next.js 16（App Router，Turbopack）+ React 19 + Tailwind CSS v4 桌面端应用，UI 逐页移植自 `../ui-原型图/prototype/`。

## 开发

```bash
pnpm dev   # 开发服务器
pnpm build # 生产构建
pnpm lint  # ESLint
```

### 后端地址

页面在服务端直接请求后端，浏览器不直连，因此不受 CORS 限制，也无需 `NEXT_PUBLIC_` 前缀。

```bash
cp .env.example .env.local   # 首次拉取后执行
```

改 `.env.local` 里的 `API_BASE_URL` 指向后端（本机 `http://127.0.0.1:8000`，局域网填后端机器 IP），
dev 会热更新该变量，不必重启。缺省或填错时 `/project` 渲染 `app/project/error.tsx` 的失败页并给出提示。

`lib/api.ts` 的每个取数函数都带 `'use cache'` + `cacheLife("seconds")` + `cacheTag("projects")`：
列表页的 prefetch 与页面内反复导航不会反复打后端（5 秒内命中缓存），写操作里成对调用
`updateTag("projects")` + `refresh()` 立即失效。

上传走服务端动作，`next.config.ts` 把 `experimental.serverActions.bodySizeLimit` 抬到 `52mb`
（默认 1MB 会挡掉几 MB 的小说正文），与后端 `LEO_MAX_UPLOAD_SIZE_MB=50` 对齐；
后端扩展名白名单目前只有 `.txt`，所以新建弹窗的 `accept` 也只收 `.txt`。

## 路由约定

一级路径即功能区，页面不放在根路径下。

| 路径 | 状态 | 说明 |
| --- | --- | --- |
| `/project` | 已实现 | 剧目库列表页（tab「全部」），源码 `app/project/page.tsx`；数据来自 `GET /api/v1/projects`（服务端渲染，读 `.env.local` 的 `API_BASE_URL`，取数期间显示骨架屏）；「新建项目」走服务端动作调 `POST /api/v1/projects`（multipart，创建成功后直接进该剧目看板）；删除（单条与批量）调 `DELETE /api/v1/projects/{id}` 软删除进回收站 |
| `/project/trash` | 已实现 | 剧目库的回收站页（tab「回收站」），源码 `app/project/trash/page.tsx` + `components/trash-panel.tsx`；列表来自 `GET /api/v1/projects/recycle-bin`，恢复走 `POST /api/v1/projects/{id}/restore`，彻底删除走 `DELETE /api/v1/projects/{id}/hard`，清空走 `DELETE /api/v1/projects/recycle-bin` |
| `/` | 占位 | 仅 `redirect("/project")`，见 `app/page.tsx`；真正的首屏后续补，改写该文件即可，**不要**把剧目库搬回根路径 |
| `/project/[id]` | 已实现 | 剧目看板，源码 `app/project/[id]/page.tsx` + `components/project-hub.tsx`（原型 `project-hub.html`）；真实剧目取 `GET /api/v1/projects/{id}` + `GET /api/v1/projects/{id}/chapters`，章节状态一律「待拆解」（后端暂无生产状态接口）但标题可点进制作工作台，无角色数据时不显示选角 tab；「追更上传章节」调 `POST /api/v1/projects/{id}/chapters/upload`（先 `dry_run` 预检再落盘，覆盖策略用 `overwrite_existing`），只对后端剧目开放；`p-*` 三个原型剧目仍走 mock（含角色台账与工作台剧本） |
| `/project/[id]/chapter/[chapterId]` | 已实现 | 章节制作工作台，源码 `app/project/[id]/chapter/[chapterId]/page.tsx` + `components/chapter-studio.tsx`（原型 `chapter-studio.html`）；后端剧目取 `GET /api/v1/projects/{id}/chapters/{chapterId}`，`sentences[]` 一句一张剧本卡片、同一批数据铺「正文」对照，说话人与情绪按「旁白 / 待标定」占位；原型剧目仍由 `studioFor()` 出 mock 剧本，两个 id 都对不上时 `notFound()`；左栏三个 pane 为大纲 / 正文 / 目录，目录取 `GET /api/v1/projects/{id}/chapters` 且可直接切章 |
| `/project/[id]/export` | 未实现 | 整书母带导出中心（原型 `export-center.html`） |
| `/voice-library` | 未实现 | 全局声线资产库（原型 `voice-library.html`） |

侧边栏与卡片里指向上面「未实现」路由的链接目前是空链，属预期状态。

## 目录结构

```
app/
  layout.tsx            全局外壳：zh-CN、暗色、字体与背景
  globals.css           Tailwind 入口 + @theme 设计 token（含骨架屏用的 `--animate-shimmer`）
  page.tsx              根路径占位跳转
  project/page.tsx      剧目库列表页（服务端取数 + Suspense 骨架）
  project/error.tsx     后端不可用时的失败页与重试
  project/trash/page.tsx 回收站页
  project/[id]/page.tsx 剧目看板页（原型剧目走 mock，其余取后端项目详情与章节大纲）
  project/[id]/chapter/[chapterId]/page.tsx 章节制作工作台页
components/
  modal.tsx             弹窗外壳（遮罩、Esc/点击关闭、标题栏）与表单样式 token；size="lg" 给宽弹窗 + 内部滚动 + footer 插槽
  toast.tsx             轻提示（ToastProvider + useToast）：文案可为 ReactNode，第三参自定义停留时长，默认 2.5 秒
  sidebar.tsx           左侧导航（「我的剧目」分组下平铺剧目，条目用 FileAudio 图标；剧目库列表页传入后端数据）
  top-bar.tsx           顶栏
  floating-actions.tsx  右缘悬浮按钮
  library-tabs.tsx      剧目库与回收站共用的一级 tab（两条独立路由，用 Link 跳转）
  project-library.tsx   列表页状态：搜索 / 排序 / 点击即删（带撤销提示条）；批量态工具栏为「批量管理」（hover 下拉：全选、删除项目）+ 独立「取消批量管理」
  trash-panel.tsx       回收站页主体：搜索 + 清空回收站 + 恢复 / 彻底删除
  trash-view.tsx        回收站视图：空态提示 + 回收卡片（含删除时间）
  project-card.tsx      剧目卡片：文件名 + 章节数/字数/更新时间，菜单为打开 / 重命名 / 删除（红字） / 批量管理；批量态封面点击改勾选
  library-skeleton.tsx  剧目库骨架屏：侧栏 + 工具栏 + 卡片网格占位，配 `animate-shimmer` 微光扫过
  import-novel-modal.tsx 新建剧目弹窗（Provider + hook + 弹窗本体）：名称 + .txt 正文，提交走 `createProjectAction`
  clone-voice-modal.tsx 克隆新音色弹窗（Provider + hook + 弹窗本体）
  chapter-import-modal.tsx 追更上传章节弹窗：选单个 .txt → dry_run 预检摘要与冲突列表 → 确认后落盘
  hub-header.tsx        剧目看板的项目级顶栏：返回书架 + 可改名标题 + 章节与副标题 + 导出入口（无进度数据时不画进度条）
  hub-chapters.tsx      看板章节状态（Provider + useHubChapters）：拆解与合成推进、持有追更上传弹窗
  project-hub.tsx       看板 tab 容器：章节流水线 / 角色阵容，持有搜索与只看已就绪；cast 为空时不渲染选角 tab
  chapter-pipeline.tsx  章节生产流水线表格：状态筛选、批量勾选、拆解与合成推进、追更上传入口
  cast-matrix.tsx       全书角色台账卡片：声线绑定与试听（Web Audio 发特征音）
  chapter-studio.tsx    工作台状态容器：段落顺序与撤销重做、当前段落、大纲/正文 pane、搜索词、配音面板
  studio-header.tsx     工作台顶栏：合并胶囊（返回 + 剧目切换 + ⌘K 搜索）与批量合成状态机
  paragraph-outline.tsx 左侧栏：段落大纲与原著正文双 pane，点击与剧本流互相定位
  script-feed.tsx       剧本流：段落卡片、拖拽与上下移排序、序号与时间轴重算、音效提示与筛选空态
  tuning-dock.tsx       右缘悬浮工具栏与展开的配音设置面板（说话人 / 模型 / 情绪 / 语速 / 混响 / Seed）
lib/
  api.ts               后端接口层：读 `API_BASE_URL`、拆 `{ code, message, data }` 包装、字段映射为视图模型
  actions.ts            服务端动作（"use server"）：新建剧目 / 移入回收站 / 恢复 / 彻底删除 / 清空，成功后 `updateTag` + `refresh()`
  projects.ts           数据模型、mock 数据与格式化函数
  audio.ts              试听特征音（Web Audio），工作台与台账共用
  scrollbar.ts          `inkScrollbar` 工具类串：暗色细胶囊滚动条，等价原型的 ::-webkit-scrollbar 处理
```

## 约定

- 样式只用 Tailwind utility 与 `@theme` token（`ink-*`、`accent`、`hairline`、`bg-card-cover`），不手写原生 CSS 规则。
- 滚动条统一用 `lib/scrollbar.ts` 的 `inkScrollbar`：`app/layout.tsx` 挂在 `html` 上管文档滚动条，页面内每个 `overflow-y-auto` 容器各自再挂一次（该样式不继承）。
- 弹窗独立成组件文件，由 Provider 持有开关、触发方通过 hook 调用，不在页面里复制弹窗 DOM。
- 剧目看板的章节列表由 `hub-chapters.tsx` 的 Provider 统一持有：顶栏统计、tab 计数与流水线表格读同一份 state。追更上传不再本地拼章节，落盘后靠 `refresh()` 让 `listChapters` 重新拉取，因此页面上 Provider 用 `key={章节 id 集合}` 强制重挂载，避免旧 state 与新 props 打架。
- 分章、同名比对、字数统计一律交给后端（`dry_run` 预检 + `overwrite_existing` 覆盖），前端不要再用正则猜章节；接口只返回数量不返回新章节标题，导入后的列表以 `listChapters` 为准。
- 图标统一用 `lucide-react`。
- 数据来源分两段：剧目库、剧目看板（项目信息 + 章节大纲）与章节工作台（正文 + 逐句）读后端，`lib/api.ts` 只在服务端调用、snake_case 映射成视图模型（`ProjectSummary` / `ProjectChapter` / `ChapterContent`）再下发组件；角色台账、章节生产状态、说话人与情绪标定仍是 `lib/projects.ts` 的 mock，后端出对应接口后逐个替换，组件层不直接依赖来源。
- 看板与工作台都是「双数据源」：`loadHub()` / `loadStudio()` 先按 id 命中原型 mock 剧目，命中不了再走后端，两条分支都归一成同一组视图类型（`HubIdentity` / `HubChapter` / `ChapterStudio`），组件层不需要知道数据来自哪边。
- 后端章节的剧本卡片由 `studioFromContent()` 生成：一句一段、原文对照与剧本流共用同一批 `sentence_id`，段时长按 4 字/秒估算，说话人统一「旁白」、情绪统一「待标定」，别在前端假造对白归属。
- 后端没有的字段不展示：列表卡片与排序只用 `name` / `filename` / `chapter_count` / `total_words` / `updated_at`（`status` 后端有值但界面不再放胶囊），看板顶栏在没有进度数据时不画进度条，别用假数据补。
- 浏览器不直连后端：读操作在 Server Component 里调 `lib/api.ts`，写操作放 `lib/actions.ts` 的 `"use server"` 动作里（后端未开 CORS），动作返回 `{ ok, message }` 而不是抛错。
- 删除是「点击即删」，没有二次确认弹窗：先乐观摘掉卡片、接口失败再回滚，成功后弹 6 秒提示条 `已删除，撤销 或 在 回收站 寻找`；撤销调恢复接口并把卡片放回列表，点「回收站」跳 `/project/trash`。
- 「全部」和「回收站」是两条路由、两个页面，共用 `library-tabs.tsx`；回收站数据全部来自后端（软删除保留正文与章节，可恢复；`/hard` 与清空才是不可逆物理删除），前端不再用 sessionStorage 存状态。
- 所有写操作都走 `lib/actions.ts` 的 `"use server"` 动作，动作里成对调用 `updateTag("projects")` + `refresh()`：前者作废服务端缓存，后者让浏览器丢掉已缓存的 RSC payload，两条路由互相切换时不必手动刷新。
- 列表组件不再把 props 复制进 state（`ProjectLibrary` 收 `projects`、`TrashPanel` 收 `items`）：本地只保留「正在处理中的 id」这类乐观覆盖，动作返回后随新 props 自动收敛，避免出现两份数据源。
- 剧目库与回收站之间的 tab 切换、以及指向剧目看板的卡片链接都 `prefetch={false}`：这些路由的数据是请求时取的，预取只会让切过去时看到旧 payload。
- 服务端取数的路由用 `Suspense` + 骨架屏占住版式（`components/library-skeleton.tsx`），不要写「正在加载…」文字；骨架的容器尺寸、栅格列数要和真实卡片一致，数据到位时不跳版。
- 批量管理的入口在卡片「···」菜单的「批量管理」项：点击后进入批量态并自动勾选该卡片，封面点击改勾选、显示`已选 N 项`。批量态下工具栏为`批量管理`(accent 高亮) + `取消批量管理` 两个按钮，前者鼠标悬停弹出下拉承载`全选/取消全选`与`删除项目`（作用于已勾选项，未选时禁用）；取消批量管理是工具栏上的独立按钮，不放进下拉。批量删除与单条删除走同一条「点击即删 + 撤销提示条」逻辑。
- 危险项只用红色文字与红色图标（`text-red-400`），底色仍走中性 `hover:bg-white/5`，不要红色背景；红白两色要写成两个完整 class 串（`menuRow` / `menuRowDanger`），拼在一条 `text-white text-red-400` 里会被 Tailwind 的产物顺序覆盖成白色。
- 原型页与 desktop 路由一一对应，每个原型页核对通过后再单独移植成一个独立页面/组件文件；不要把新页面写进已有页面的文件里。
- 全局层页面（`/project`）用 `Sidebar` + `TopBar`；单书上下文层的剧目看板与章节工作台按原型自带的项目顶栏（`hub-header.tsx` / `studio-header.tsx`）呈现，不再叠全局外壳。
- 章节工作台是 `h-screen` 满屏布局：页面自身不滚动，大纲、正文与剧本流各自内部滚动。
- 左栏与剧本流互相定位：点大纲/正文条目滚到对应卡片，点卡片只滚到左栏**当前 pane** 的对应条目（大纲态就停在大纲，不切成正文）；pane 由用户手动切换，跳转用 `outline-*` / `source-*` / 卡片 id 三类 DOM 锚点。
- 左栏第三个 pane「目录」来自 `GET /projects/{id}/chapters`（`studioTocFor()` 把大纲映射成行，当前章高亮并在进入 pane 时滚到该行），点任意一行 `router.push` 切到该章工作台；原型剧目没有该接口，由看板 mock 章节转成同构的 `ProjectChapter` 喂同一个映射函数。
- 左栏当前 pane 记在 `chapter-studio.tsx` 的模块变量 `rememberedPane` 里：切章会让工作台子树重挂载，靠它停在用户上次看的那一栏（不闪、不水合错位），整页刷新才回到大纲。别改成 `useState` 初值读 `sessionStorage`/`useSearchParams`，那会和服务端 HTML 对不上。
