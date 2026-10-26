# VoiceNovel Studio · desktop

Next.js 16（App Router，Turbopack）+ React 19 + Tailwind CSS v4 桌面端应用，UI 由 `../ui-原型图/prototype/index.html` 移植而来。

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
| `/project/[id]` | 未实现 | 剧目详情 / 看板（原型 `project-hub.html`） |
| `/folder/[id]` | 未实现 | 文件夹视图 |
| `/voice-library` | 未实现 | 全局声线资产库（原型 `voice-library.html`） |

侧边栏与卡片里指向上面「未实现」路由的链接目前是空链，属预期状态。

## 目录结构

```
app/
  layout.tsx            全局外壳：zh-CN、暗色、字体与背景
  globals.css           Tailwind 入口 + @theme 设计 token
  page.tsx              根路径占位跳转
  project/page.tsx      剧目库列表页
components/
  modal.tsx             弹窗外壳（遮罩、Esc/点击关闭、标题栏）与表单样式 token
  sidebar.tsx           左侧导航（文件夹用 Folder、剧目文件用 FileAudio 区分）
  top-bar.tsx           顶栏
  project-library.tsx   列表状态：tab / 搜索 / 只看收藏 / 排序 / 新建文件夹
  project-card.tsx      剧目与文件夹卡片，含卡片操作菜单
  waveform.tsx          封面声波，种子伪随机保证 SSR 与水合一致
  import-novel-modal.tsx 导入小说弹窗（Provider + hook + 弹窗本体）
  create-folder-modal.tsx 新建文件夹弹窗，仅一个名称输入框
lib/
  projects.ts           数据模型、mock 数据与格式化函数
```

## 约定

- 样式只用 Tailwind utility 与 `@theme` token（`ink-*`、`accent`、`hairline`、`bg-card-cover`），不手写原生 CSS 规则。
- 弹窗独立成组件文件，由 Provider 持有开关、触发方通过 hook 调用，不在页面里复制弹窗 DOM。
- 图标统一用 `lucide-react`。
- 数据目前来自 `lib/projects.ts` 的 mock，接后端时替换该文件即可，组件层不直接依赖来源。
