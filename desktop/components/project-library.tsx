"use client";

import { Filter, Inbox, Plus, Search } from "lucide-react";
import { useRouter } from "next/navigation";
import { useEffect, useMemo, useState } from "react";
import { useImportNovelModal } from "@/components/import-novel-modal";
import { ProjectCard, type CardAction } from "@/components/project-card";
import {
  sortProjects,
  sortOptions,
  type ProjectEntry,
  type SortKey,
} from "@/lib/projects";

export function ProjectLibrary({ initialProjects }: { initialProjects: ProjectEntry[] }) {
  const router = useRouter();
  const { open: openImportModal } = useImportNovelModal();
  const [projects, setProjects] = useState(initialProjects);
  const [keyword, setKeyword] = useState("");
  const [sortKey, setSortKey] = useState<SortKey | null>(null);
  const [sortMenuOpen, setSortMenuOpen] = useState(false);
  const [menuId, setMenuId] = useState<string | null>(null);
  const [renamingId, setRenamingId] = useState<string | null>(null);

  useEffect(() => {
    const dismiss = (event: MouseEvent) => {
      if ((event.target as Element).closest("[data-menu]")) return;
      setMenuId(null);
      setSortMenuOpen(false);
    };
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        setMenuId(null);
        setSortMenuOpen(false);
      }
    };
    document.addEventListener("mousedown", dismiss);
    document.addEventListener("keydown", onKeyDown);
    return () => {
      document.removeEventListener("mousedown", dismiss);
      document.removeEventListener("keydown", onKeyDown);
    };
  }, []);

  const visible = useMemo(() => {
    const needle = keyword.trim().toLowerCase();
    const matched = projects.filter(
      (project) => !needle || project.title.toLowerCase().includes(needle),
    );
    return sortKey ? sortProjects(matched, sortKey) : matched;
  }, [projects, keyword, sortKey]);

  function update(id: string, patch: { title?: string }) {
    setProjects((prev) =>
      prev.map((project) => (project.id === id ? { ...project, ...patch } : project)),
    );
  }

  function handleAction(id: string, action: CardAction) {
    setMenuId(null);
    if (action === "open") {
      router.push(`/project/${id}`);
      return;
    }
    if (action === "rename") setRenamingId(id);
  }

  return (
    <>
      <div className="mt-6 flex flex-wrap items-center justify-between gap-3">
        <div className="flex items-center gap-1">
          <span className="rounded-lg px-2.5 py-1.5 text-[15px] font-semibold text-white">
            全部
          </span>
        </div>

        <div className="flex items-center gap-2">
          <div className="relative" data-menu>
            <button
              type="button"
              title="排序"
              onClick={() => setSortMenuOpen((value) => !value)}
              className="rounded-full border border-hairline bg-ink-800 p-2 text-white/50 transition-colors hover:text-white"
            >
              <Filter className="h-4 w-4" />
            </button>
            {sortMenuOpen && (
              <div className="absolute right-0 top-11 z-20 w-40 overflow-hidden rounded-xl border border-hairline bg-ink-750 py-1 shadow-2xl shadow-black/60">
                {sortOptions.map((option) => (
                  <button
                    key={option.key}
                    type="button"
                    onClick={() => {
                      setSortKey(option.key);
                      setSortMenuOpen(false);
                    }}
                    className={`flex w-full items-center justify-between px-3 py-2 text-[12.5px] hover:bg-white/5 ${
                      sortKey === option.key ? "text-accent-400" : "text-white/70"
                    }`}
                  >
                    {option.label}
                  </button>
                ))}
              </div>
            )}
          </div>

          <div className="relative">
            <Search className="pointer-events-none absolute left-3.5 top-2.5 h-4 w-4 text-white/30" />
            <input
              type="text"
              value={keyword}
              onChange={(e) => setKeyword(e.target.value)}
              placeholder="搜索剧目"
              className="w-[164px] rounded-full border border-hairline bg-ink-800 py-2 pl-9 pr-3 text-[12.5px] text-white placeholder:text-white/30 focus:border-accent/60 focus:outline-none xl:w-[220px]"
            />
          </div>
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
            新建项目
          </button>
        </div>

        {visible.map((project) => (
          <ProjectCard
            key={project.id}
            project={project}
            menuOpen={menuId === project.id}
            renaming={renamingId === project.id}
            onToggleMenu={(id) => setMenuId((current) => (current === id ? null : id))}
            onAction={handleAction}
            onRename={(id, title) => {
              if (title) update(id, { title });
              setRenamingId(null);
            }}
          />
        ))}

        {visible.length === 0 && (
          <div className="min-h-[236px] rounded-2xl border border-dashed border-white/10 bg-ink-850 p-8 sm:col-span-2 xl:col-span-3">
            <div className="flex h-full flex-col items-center justify-center text-center">
              <Inbox className="h-7 w-7 text-white/20" />
              <p className="mt-3 text-[13px] text-white/45">没有符合条件的剧目</p>
            </div>
          </div>
        )}
      </div>
    </>
  );
}
