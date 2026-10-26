"use client";

import { ArrowLeft, FolderOpen, Plus, Search } from "lucide-react";
import Link from "next/link";
import { useRouter } from "next/navigation";
import { useEffect, useMemo, useState } from "react";
import { ProjectCard, type CardAction } from "@/components/project-card";
import { useImportNovelModal } from "@/components/import-novel-modal";
import {
  projectsInFolder,
  relativeTime,
  type FolderEntry,
  type LibraryEntry,
} from "@/lib/projects";

export function FolderView({
  folder,
  entries: initialEntries,
}: {
  folder: FolderEntry;
  entries: LibraryEntry[];
}) {
  const router = useRouter();
  const { open: openImportModal } = useImportNovelModal();
  const [entries, setEntries] = useState(initialEntries);
  const [keyword, setKeyword] = useState("");
  const [menuId, setMenuId] = useState<string | null>(null);
  const [renamingId, setRenamingId] = useState<string | null>(null);

  useEffect(() => {
    const dismiss = (event: MouseEvent) => {
      if ((event.target as Element).closest("[data-menu]")) return;
      setMenuId(null);
    };
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") setMenuId(null);
    };
    document.addEventListener("mousedown", dismiss);
    document.addEventListener("keydown", onKeyDown);
    return () => {
      document.removeEventListener("mousedown", dismiss);
      document.removeEventListener("keydown", onKeyDown);
    };
  }, []);

  const folderProjects = projectsInFolder(entries, folder.id);
  const visible = useMemo(() => {
    const needle = keyword.trim().toLowerCase();
    return folderProjects.filter(
      (project) => !needle || project.title.toLowerCase().includes(needle),
    );
  }, [folderProjects, keyword]);

  function update(id: string, patch: { title?: string; fav?: boolean; trashed?: boolean }) {
    setEntries((prev) =>
      prev.map((entry) =>
        entry.id === id ? ({ ...entry, ...patch } as LibraryEntry) : entry,
      ),
    );
  }

  function handleAction(id: string, action: CardAction) {
    setMenuId(null);
    if (action === "open") {
      router.push(`/project/${id}`);
      return;
    }
    const entry = entries.find((item) => item.id === id);
    if (!entry) return;
    if (action === "rename") setRenamingId(id);
    if (action === "fav") update(id, { fav: !entry.fav });
    if (action === "trash") update(id, { trashed: true });
    if (action === "restore") update(id, { trashed: false });
    if (action === "purge") setEntries((prev) => prev.filter((item) => item.id !== id));
  }

  const lastEdited = folderProjects.length
    ? Math.min(...folderProjects.map((project) => project.editedHoursAgo))
    : folder.editedHoursAgo;

  return (
    <main className="flex-1 px-5 pb-16 pt-8 sm:px-8">
      <div className="flex flex-wrap items-start justify-between gap-4">
        <div className="flex items-center gap-4">
          <Link
            href="/project"
            title="返回剧目库"
            className="flex h-10 w-10 shrink-0 items-center justify-center rounded-xl border border-hairline bg-ink-800 text-white/60 transition-colors hover:border-white/20 hover:text-white"
          >
            <ArrowLeft className="h-[18px] w-[18px]" />
          </Link>
          <div>
            <h1 className="flex items-center gap-2.5 text-[26px] font-bold leading-none tracking-tight text-white">
              <FolderOpen className="h-6 w-6 text-amber-300/60" />
              {folder.title}
            </h1>
            <p className="mt-2 text-[12px] text-white/35">
              {folderProjects.length} 部剧目 · 最近编辑 {relativeTime(lastEdited)}
            </p>
          </div>
        </div>

        <div className="relative">
          <Search className="pointer-events-none absolute left-3.5 top-2.5 h-4 w-4 text-white/30" />
          <input
            type="text"
            value={keyword}
            onChange={(e) => setKeyword(e.target.value)}
            placeholder="搜索剧目"
            className="w-[200px] rounded-full border border-hairline bg-ink-800 py-2 pl-9 pr-3 text-[12.5px] text-white placeholder:text-white/30 focus:border-accent/60 focus:outline-none xl:w-[260px]"
          />
        </div>
      </div>

      <div className="mt-6 grid grid-cols-1 gap-5 sm:grid-cols-2 xl:grid-cols-3 2xl:grid-cols-4">
        <div className="flex min-h-[236px] items-center justify-center rounded-2xl border border-dashed border-white/10 bg-ink-850">
          <button
            type="button"
            onClick={openImportModal}
            className="flex items-center gap-2 rounded-full border border-hairline bg-ink-700 px-5 py-2.5 text-[13px] font-medium text-white/80 transition-colors hover:bg-ink-600 hover:text-white"
          >
            <Plus className="h-4 w-4" />
            新建剧目
          </button>
        </div>

        {visible.map((entry) => (
          <ProjectCard
            key={entry.id}
            entry={entry}
            entries={entries}
            menuOpen={menuId === entry.id}
            renaming={renamingId === entry.id}
            onToggleMenu={(id) => setMenuId((current) => (current === id ? null : id))}
            onAction={handleAction}
            onRename={(id, title) => {
              if (title) update(id, { title });
              setRenamingId(null);
            }}
          />
        ))}

        {visible.length === 0 && (
          <div className="flex min-h-[120px] items-center justify-center py-24 sm:col-span-2 xl:col-span-3">
            <p className="text-[13px] text-white/35">
              {keyword ? "没有符合条件的剧目" : "此文件夹暂无剧目"}
            </p>
          </div>
        )}
      </div>
    </main>
  );
}
