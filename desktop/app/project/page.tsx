import { Keyboard, MessageCircleMore } from "lucide-react";
import { ImportNovelModalProvider } from "@/components/import-novel-modal";
import { ProjectLibrary } from "@/components/project-library";
import { Sidebar } from "@/components/sidebar";
import { TopBar } from "@/components/top-bar";
import { initialEntries } from "@/lib/projects";

const floatingButton =
  "flex h-9 w-9 items-center justify-center rounded-full border border-hairline bg-ink-800 text-white/50 transition-colors hover:text-white";

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

      <div className="fixed right-4 top-1/2 z-30 hidden -translate-y-1/2 flex-col gap-2 2xl:flex">
        <button type="button" title="意见反馈" className={floatingButton}>
          <MessageCircleMore className="h-4 w-4" />
        </button>
        <button type="button" title="快捷键" className={floatingButton}>
          <Keyboard className="h-4 w-4" />
        </button>
      </div>
    </ImportNovelModalProvider>
  );
}
