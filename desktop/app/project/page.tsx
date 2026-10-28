import { Suspense } from "react";
import { FloatingActions } from "@/components/floating-actions";
import { ImportNovelModalProvider } from "@/components/import-novel-modal";
import { LibrarySkeleton } from "@/components/library-skeleton";
import { ProjectLibrary } from "@/components/project-library";
import { Sidebar } from "@/components/sidebar";
import { ToastProvider } from "@/components/toast";
import { TopBar } from "@/components/top-bar";
import { listProjects } from "@/lib/api";

// 剧目列表来自后端，进入本页需要等服务端取数
export const instant = false;

async function LibraryShell() {
  const projects = await listProjects();

  return (
    <div className="flex min-h-screen">
      <Sidebar projects={projects} />

      <div className="flex min-w-0 flex-1 flex-col lg:ml-[236px]">
        <TopBar />
        <main className="flex-1 px-5 pb-16 pt-8 sm:px-8">
          <h1 className="text-[26px] font-bold leading-none tracking-tight text-white">
            我的剧目
          </h1>
          <ProjectLibrary initialProjects={projects} />
        </main>
      </div>
    </div>
  );
}

export default function ProjectPage() {
  return (
    <ToastProvider>
      <ImportNovelModalProvider>
        <Suspense fallback={<LibrarySkeleton />}>
          <LibraryShell />
        </Suspense>

        <FloatingActions />
      </ImportNovelModalProvider>
    </ToastProvider>
  );
}
