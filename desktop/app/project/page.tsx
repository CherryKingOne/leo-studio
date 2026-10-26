import { FloatingActions } from "@/components/floating-actions";
import { ImportNovelModalProvider } from "@/components/import-novel-modal";
import { ProjectLibrary } from "@/components/project-library";
import { Sidebar } from "@/components/sidebar";
import { TopBar } from "@/components/top-bar";
import { initialEntries } from "@/lib/projects";

export default function ProjectPage() {
  return (
    <ImportNovelModalProvider>
      <div className="flex min-h-screen">
        <Sidebar entries={initialEntries} />

        <div className="flex min-w-0 flex-1 flex-col lg:ml-[236px]">
          <TopBar />
          <main className="flex-1 px-5 pb-16 pt-8 sm:px-8">
            <h1 className="text-[26px] font-bold leading-none tracking-tight text-white">
              我的剧目
            </h1>
            <ProjectLibrary initialEntries={initialEntries} />
          </main>
        </div>
      </div>

      <FloatingActions />
    </ImportNovelModalProvider>
  );
}
