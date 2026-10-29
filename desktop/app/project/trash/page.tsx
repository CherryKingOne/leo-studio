import type { Metadata } from "next";
import { Suspense } from "react";
import { FloatingActions } from "@/components/floating-actions";
import { ImportNovelModalProvider } from "@/components/import-novel-modal";
import { LibrarySkeleton } from "@/components/library-skeleton";
import { Sidebar } from "@/components/sidebar";
import { ToastProvider } from "@/components/toast";
import { TopBar } from "@/components/top-bar";
import { TrashPanel } from "@/components/trash-panel";
import { listProjects } from "@/lib/api";

export const metadata: Metadata = { title: "回收站 · 剧目库" };

// 侧边栏剧目列表来自后端，进入本页需要等服务端取数
export const instant = false;

async function TrashShell() {
  const projects = await listProjects();

  return (
    <div className="flex min-h-screen">
      <Sidebar projects={projects} />

      <div className="flex min-w-0 flex-1 flex-col lg:ml-[236px]">
        <TopBar />
        <TrashPanel />
      </div>
    </div>
  );
}

export default function TrashPage() {
  return (
    <ToastProvider>
      <ImportNovelModalProvider>
        <Suspense fallback={<LibrarySkeleton />}>
          <TrashShell />
        </Suspense>

        <FloatingActions />
      </ImportNovelModalProvider>
    </ToastProvider>
  );
}
