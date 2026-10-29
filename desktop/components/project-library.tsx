"use client";

import { Filter, Inbox, ListChecks, Plus, Search, Trash2, Undo2, X } from "lucide-react";
import { useRouter } from "next/navigation";
import { useEffect, useMemo, useState } from "react";
import { useImportNovelModal } from "@/components/import-novel-modal";
import { LibraryTabs } from "@/components/library-tabs";
import { ProjectCard, type CardAction } from "@/components/project-card";
import { useToast } from "@/components/toast";
import { removeProjects, restoreProjects } from "@/lib/actions";
import {
  sortProjects,
  sortOptions,
  type ProjectSummary,
  type SortKey,
} from "@/lib/projects";

const toolbarButton =
  "flex items-center gap-1.5 rounded-full border border-hairline bg-ink-800 px-3.5 py-2 text-[12.5px] text-white/70 transition-colors hover:border-white/20 hover:text-white disabled:cursor-not-allowed disabled:opacity-40";

const menuItem =
  "flex w-full items-center gap-2.5 px-3 py-2 text-left text-[12.5px] text-white transition-colors hover:bg-white/5 disabled:cursor-not-allowed disabled:opacity-40";

const menuItemDanger =
  "flex w-full items-center gap-2.5 px-3 py-2 text-left text-[12.5px] text-red-400 transition-colors hover:bg-white/5 disabled:cursor-not-allowed disabled:opacity-40";

const toastLink =
  "flex items-center gap-1 font-medium text-accent-400 transition-colors hover:text-accent";

export function ProjectLibrary({
  projects,
  trashCount,
}: {
  projects: ProjectSummary[];
  trashCount: number;
}) {
  const router = useRouter();
  const { show } = useToast();
  const { open: openImportModal } = useImportNovelModal();
  // 列表本身只来自服务端 props；本地只留「乐观删除的 id」与「改名草稿」两类覆盖，
  // 动作完成后 refresh() 会带回新 props，覆盖随即清掉，避免和后端数据打架
  const [hidden, setHidden] = useState<string[]>([]);
  const [pendingTrash, setPendingTrash] = useState(0);
  const [renames, setRenames] = useState<Record<string, string>>({});
  const [keyword, setKeyword] = useState("");
  const [sortKey, setSortKey] = useState<SortKey | null>(null);
  const [sortMenuOpen, setSortMenuOpen] = useState(false);
  const [menuId, setMenuId] = useState<string | null>(null);
  const [renamingId, setRenamingId] = useState<string | null>(null);
  const [batchMode, setBatchMode] = useState(false);
  const [batchMenuOpen, setBatchMenuOpen] = useState(false);
  const [selected, setSelected] = useState<string[]>([]);

  useEffect(() => {
    const dismiss = (event: MouseEvent) => {
      if ((event.target as Element).closest("[data-menu]")) return;
      setMenuId(null);
      setSortMenuOpen(false);
      setBatchMenuOpen(false);
    };
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        setMenuId(null);
        setSortMenuOpen(false);
        setBatchMenuOpen(false);
      }
    };
    document.addEventListener("mousedown", dismiss);
    document.addEventListener("keydown", onKeyDown);
    return () => {
      document.removeEventListener("mousedown", dismiss);
      document.removeEventListener("keydown", onKeyDown);
    };
  }, []);

  const list = useMemo(() => {
    const dropped = new Set(hidden);
    return projects
      .filter((project) => !dropped.has(project.id))
      .map((project) =>
        renames[project.id] ? { ...project, title: renames[project.id] } : project,
      );
  }, [projects, hidden, renames]);

  const visible = useMemo(() => {
    const needle = keyword.trim().toLowerCase();
    const matched = list.filter(
      (project) => !needle || project.title.toLowerCase().includes(needle),
    );
    return sortKey ? sortProjects(matched, sortKey) : matched;
  }, [list, keyword, sortKey]);

  function handleAction(id: string, action: CardAction) {
    setMenuId(null);
    if (action === "open") {
      router.push(`/project/${id}`);
      return;
    }
    if (action === "rename") {
      setRenamingId(id);
      return;
    }
    if (action === "delete") {
      const project = list.find((item) => item.id === id);
      if (project) void moveToTrash([project]);
      return;
    }
    setBatchMode(true);
    setSelected([id]);
  }

  function toggleSelect(id: string) {
    setSelected((prev) =>
      prev.includes(id) ? prev.filter((item) => item !== id) : [...prev, id],
    );
  }

  function exitBatch() {
    setBatchMode(false);
    setSelected([]);
  }

  /** 移入回收站：先乐观摘卡，动作里 refresh() 带回新 props 后再撤掉覆盖 */
  async function moveToTrash(targets: ProjectSummary[]) {
    if (!targets.length) return;

    const ids = targets.map((project) => project.id);
    const dropped = new Set(ids);
    setHidden((prev) => [...prev, ...ids]);
    setPendingTrash(ids.length);
    setSelected((prev) => prev.filter((id) => !dropped.has(id)));
    setBatchMenuOpen(false);

    const result = await removeProjects(ids);
    setHidden((prev) => prev.filter((id) => !dropped.has(id)));
    setPendingTrash(0);
    if (!result.ok) {
      show(result.message, "info");
      return;
    }

    show(
      <span className="flex flex-wrap items-center gap-1.5">
        已删除，
        <button type="button" onClick={() => void restore(ids)} className={toastLink}>
          <Undo2 className="h-3.5 w-3.5" />
          撤销
        </button>
        或 在
        <button type="button" onClick={() => router.push("/project/trash")} className={toastLink}>
          <Trash2 className="h-3.5 w-3.5" />
          回收站
        </button>
        寻找
      </span>,
      "info",
      6000,
    );
  }

  /** 撤销：调恢复接口，新列表由 refresh() 带回 */
  async function restore(ids: string[]) {
    const result = await restoreProjects(ids);
    if (!result.ok) {
      show(result.message, "info");
      return;
    }
    show(`已恢复 ${ids.length} 部剧目`, "success");
  }

  const allSelected = visible.length > 0 && visible.every((project) => selected.includes(project.id));

  return (
    <>
      <div className="mt-6 flex flex-wrap items-center justify-between gap-3">
        <div className="flex items-center gap-3">
          <LibraryTabs trashCount={trashCount + pendingTrash} />
          {batchMode && selected.length > 0 && (
            <span className="font-mono text-[11.5px] text-white/40">
              已选 {selected.length} 项
            </span>
          )}
        </div>

        <div className="flex items-center gap-2">
          {batchMode && (
            <div
              className="relative"
              data-menu
              onMouseEnter={() => setBatchMenuOpen(true)}
              onMouseLeave={() => setBatchMenuOpen(false)}
            >
              <button
                type="button"
                onClick={() => setMenuId(null)}
                className={`${toolbarButton} border-accent/30 bg-accent/15 text-accent-400`}
              >
                <ListChecks className="h-3.5 w-3.5" />
                批量管理
              </button>

              {batchMenuOpen && (
                <div className="absolute right-0 top-11 z-20 w-40 overflow-hidden rounded-xl border border-hairline bg-ink-750 py-1 shadow-2xl shadow-black/60">
                  <button
                    type="button"
                    disabled={visible.length === 0}
                    onClick={() =>
                      setSelected(allSelected ? [] : visible.map((project) => project.id))
                    }
                    className={menuItem}
                  >
                    <ListChecks className="h-3.5 w-3.5" />
                    {allSelected ? "取消全选" : "全选"}
                  </button>
                  <button
                    type="button"
                    disabled={selected.length === 0}
                    onClick={() => void moveToTrash(list.filter((project) => selected.includes(project.id)))}
                    className={menuItemDanger}
                  >
                    <Trash2 className="h-3.5 w-3.5" />
                    删除项目
                  </button>
                </div>
              )}
            </div>
          )}

          {batchMode && (
            <button type="button" onClick={exitBatch} className={toolbarButton}>
              <X className="h-3.5 w-3.5" />
              取消批量管理
            </button>
          )}

          {batchMode && (
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
          )}

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
        {!batchMode && (
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
        )}

        {visible.map((project) => (
          <ProjectCard
            key={project.id}
            project={project}
            menuOpen={menuId === project.id}
            renaming={renamingId === project.id}
            batchMode={batchMode}
            selected={selected.includes(project.id)}
            onToggleMenu={(id) => setMenuId((current) => (current === id ? null : id))}
            onAction={handleAction}
            onRename={(id, title) => {
              // 后端还没有改名接口，改名先留在本地覆盖层
              if (title) setRenames((prev) => ({ ...prev, [id]: title }));
              setRenamingId(null);
            }}
            onToggleSelect={toggleSelect}
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
