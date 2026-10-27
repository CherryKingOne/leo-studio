# VoiceNovel Studio · desktop

Next.js 16（App Router，Turbopack）+ React 19 + Tailwind CSS v4 桌面端应用，UI 逐页移植自 `../ui-原型图/prototype/`。

## 开发

```bash
pnpm dev   # 开发服务器
pnpm build # 生产构建
pnpm lint  # ESLint
```

## 路由约定

一级路径即功能区，页面不放在根路径下。

| 路径 | 状态 | 说明 |
| --- | --- | --- |
| `/project` | 已实现 | 剧目库列表页，源码 `app/project/page.tsx` |
| `/` | 占位 | 仅 `redirect("/project")`，见 `app/page.tsx`；真正的首屏后续补，改写该文件即可，**不要**把剧目库搬回根路径 |
| `/project/[id]` | 已实现 | 剧目看板，源码 `app/project/[id]/page.tsx` + `components/project-hub.tsx`（原型 `project-hub.html`）；`generateStaticParams` 预渲染已知剧目，未知 id 走 `notFound()` |
| `/folder/[id]` | 已实现 | 剧目文件夹视图，源码 `app/folder/[id]/page.tsx`（原型 `folder-view.html`）；`generateStaticParams` 预渲染已知文件夹，未知 id 走 `notFound()` |
| `/project/[id]/chapter/[chapterId]` | 已实现 | 章节制作工作台，源码 `app/project/[id]/chapter/[chapterId]/page.tsx` + `components/chapter-studio.tsx`（原型 `chapter-studio.html`）；`generateStaticParams` 预渲染看板中 `openable` 的章节，未知 id 走 `notFound()`；未逐句核对过的章节由 `studioFor()` 铺占位剧本 |
| `/project/[id]/export` | 未实现 | 整书母带导出中心（原型 `export-center.html`） |
| `/voice-library` | 未实现 | 全局声线资产库（原型 `voice-library.html`） |

侧边栏与卡片里指向上面「未实现」路由的链接目前是空链，属预期状态。

## 目录结构

```
app/
  layout.tsx            全局外壳：zh-CN、暗色、字体与背景
  globals.css           Tailwind 入口 + @theme 设计 token
  page.tsx              根路径占位跳转
  project/page.tsx      剧目库列表页
  project/[id]/page.tsx 剧目看板页
  project/[id]/chapter/[chapterId]/page.tsx 章节制作工作台页
  folder/[id]/page.tsx  剧目文件夹页
components/
  modal.tsx             弹窗外壳（遮罩、Esc/点击关闭、标题栏）与表单样式 token
  toast.tsx             轻提示（ToastProvider + useToast，2.5 秒自动消失）
  sidebar.tsx           左侧导航（文件夹用 Folder、剧目文件用 FileAudio 区分；activeFolderId 高亮当前文件夹）
  top-bar.tsx           顶栏
  floating-actions.tsx  右缘悬浮按钮
  project-library.tsx   列表状态：tab / 搜索 / 只看收藏 / 排序 / 新建文件夹
  folder-view.tsx       文件夹视图：返回 + 文件夹名 + 搜索 + 该文件夹内剧目网格
  project-card.tsx      剧目与文件夹卡片，含卡片操作菜单
  waveform.tsx          封面声波，种子伪随机保证 SSR 与水合一致
  import-novel-modal.tsx 导入小说弹窗（Provider + hook + 弹窗本体）
  create-folder-modal.tsx 新建文件夹弹窗，仅一个名称输入框
  clone-voice-modal.tsx 克隆新音色弹窗（Provider + hook + 弹窗本体）
  hub-header.tsx        剧目看板的项目级顶栏：返回书架 + 可改名标题 + 全书进度 + 导出入口
  project-hub.tsx       看板 tab 容器：章节流水线 / 角色阵容，持有搜索与只看已就绪
  chapter-pipeline.tsx  章节生产流水线表格：状态筛选、批量勾选、拆解与合成推进
  cast-matrix.tsx       全书角色台账卡片：声线绑定与试听（Web Audio 发特征音）
  chapter-studio.tsx    工作台状态容器：段落顺序与撤销重做、当前段落、大纲/正文 pane、搜索词、配音面板
  studio-header.tsx     工作台顶栏：合并胶囊（返回 + 剧目切换 + ⌘K 搜索）与批量合成状态机
  paragraph-outline.tsx 左侧栏：段落大纲与原著正文双 pane，点击与剧本流互相定位
  script-feed.tsx       剧本流：段落卡片、拖拽与上下移排序、序号与时间轴重算、音效提示与筛选空态
  tuning-dock.tsx       右缘悬浮工具栏与展开的配音设置面板（说话人 / 模型 / 情绪 / 语速 / 混响 / Seed）
lib/
  projects.ts           数据模型、mock 数据与格式化函数
  audio.ts              试听特征音（Web Audio），工作台与台账共用
  scrollbar.ts          `inkScrollbar` 工具类串：暗色细胶囊滚动条，等价原型的 ::-webkit-scrollbar 处理
```

## 约定

- 样式只用 Tailwind utility 与 `@theme` token（`ink-*`、`accent`、`hairline`、`bg-card-cover`），不手写原生 CSS 规则。
- 滚动条统一用 `lib/scrollbar.ts` 的 `inkScrollbar`：`app/layout.tsx` 挂在 `html` 上管文档滚动条，页面内每个 `overflow-y-auto` 容器各自再挂一次（该样式不继承）。
- 弹窗独立成组件文件，由 Provider 持有开关、触发方通过 hook 调用，不在页面里复制弹窗 DOM。
- 图标统一用 `lucide-react`。
- 数据目前来自 `lib/projects.ts` 的 mock，接后端时替换该文件即可，组件层不直接依赖来源。
- 原型页与 desktop 路由一一对应，每个原型页核对通过后再单独移植成一个独立页面/组件文件；不要把新页面写进已有页面的文件里。
- 全局层页面（`/project`、`/folder/[id]`）用 `Sidebar` + `TopBar`；单书上下文层的剧目看板与章节工作台按原型自带的项目顶栏（`hub-header.tsx` / `studio-header.tsx`）呈现，不再叠全局外壳。
- 章节工作台是 `h-screen` 满屏布局：页面自身不滚动，大纲、正文与剧本流各自内部滚动。
