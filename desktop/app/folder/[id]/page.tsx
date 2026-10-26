import type { Metadata } from "next";
import { notFound } from "next/navigation";
import { FloatingActions } from "@/components/floating-actions";
import { FolderView } from "@/components/folder-view";
import { ImportNovelModalProvider } from "@/components/import-novel-modal";
import { Sidebar } from "@/components/sidebar";
import { TopBar } from "@/components/top-bar";
import { initialEntries, type FolderEntry } from "@/lib/projects";

// 页面内容按 params 选文件夹，属于阻塞式路由
export const instant = false;

function findFolder(id: string) {
  return initialEntries.find(
    (entry): entry is FolderEntry => entry.kind === "folder" && entry.id === id,
  );
}

export function generateStaticParams() {
  return initialEntries
    .filter((entry) => entry.kind === "folder")
    .map((entry) => ({ id: entry.id }));
}

export async function generateMetadata(
  props: PageProps<"/folder/[id]">,
): Promise<Metadata> {
  const { id } = await props.params;
  const folder = findFolder(id);
  return {
    title: folder ? `${folder.title} · 剧目文件夹` : "剧目文件夹",
  };
}

export default async function FolderPage(props: PageProps<"/folder/[id]">) {
  const { id } = await props.params;
  const folder = findFolder(id);
  if (!folder) notFound();

  return (
    <ImportNovelModalProvider>
      <div className="flex min-h-screen">
        <Sidebar entries={initialEntries} activeFolderId={folder.id} />

        <div className="flex min-w-0 flex-1 flex-col lg:ml-[236px]">
          <TopBar />
          <FolderView folder={folder} entries={initialEntries} />
        </div>
      </div>
      <FloatingActions />
    </ImportNovelModalProvider>
  );
}
