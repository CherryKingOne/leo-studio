import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "我的剧目 · VoiceNovel AI",
  description: "导入长篇小说，自动拆章、分配声线并批量生成有声剧目",
};

export default function RootLayout({ children }: LayoutProps<"/">) {
  return (
    <html lang="zh-CN" className="dark">
      <body className="min-h-screen bg-ink-900 font-sans text-white/70 antialiased selection:bg-accent/30 selection:text-white">
        {children}
      </body>
    </html>
  );
}
