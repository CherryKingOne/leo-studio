import type { Metadata } from "next";
import { inkScrollbar } from "@/lib/scrollbar";
import "./globals.css";

export const metadata: Metadata = {
  title: "我的剧目 · VoiceNovel AI",
  description: "导入长篇小说，自动拆章、分配声线并批量生成有声剧目",
};

export default function RootLayout({ children }: LayoutProps<"/">) {
  return (
    // 翻译类浏览器插件会在水合前给 <html> 塞属性（如 data-immersive-translate-page-theme），
    // 只对这个元素关水合告警，属性差异本身不影响应用状态
    <html lang="zh-CN" className={`dark ${inkScrollbar}`} suppressHydrationWarning>
      <body className="min-h-screen bg-ink-900 font-sans text-white/70 antialiased selection:bg-accent/30 selection:text-white">
        {children}
      </body>
    </html>
  );
}
