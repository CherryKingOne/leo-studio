"use client";

import { Eraser, Search } from "lucide-react";
import { useEffect, useState } from "react";
import { LibraryTabs } from "@/components/library-tabs";
import { TrashView } from "@/components/trash-view";
import { useToast } from "@/components/toast";
import { clearAllTrash, purgeProjects, restoreProjects } from "@/lib/actions";
import type { TrashedProject } from "@/lib/projects";

export function TrashPanel({ items }: { items: TrashedProject[] }) {
  const { show } = useToast();
  // 列表只来自服务端 props，本地只留正在处理的条目做乐观隐藏，动作完成后 refresh() 带回新列表
  const [pending, setPending] = useState<string[]>([]);
  const [menuId, setMenuId] = useState<string | null>(null);
  const [keyword, setKeyword] = useState("");

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

  const needle = keyword.trim().toLowerCase();
  const visible = items.filter(
    (item) => !pending.includes(item.id) && (!needle || item.title.toLowerCase().includes(needle)),
  );

  async function run(ids: string[], action: () => Promise<{ ok: boolean; message?: string }>, done: string) {
    setMenuId(null);
    setPending((prev) => [...prev, ...ids]);
    const result = await action();
    setPending((prev) => prev.filter((id) => !ids.includes(id)));
    show(result.ok ? done : (result.message ?? "请求失败"), result.ok ? "success" : "info");
  }

  function handleAction(id: string, action: "restore" | "purge") {
    if (action === "restore") {
      void run([id], () => restoreProjects([id]), "已恢复到剧目库");
      return;
    }
    void run([id], () => purgeProjects([id]), "已彻底删除");
  }

  return (
    <main className="flex-1 px-5 pb-16 pt-8 sm:px-8">
      <h1 className="text-[26px] font-bold leading-none tracking-tight text-white">我的剧目</h1>

      <div className="mt-6 flex flex-wrap items-center justify-between gap-3">
        <LibraryTabs trashCount={items.length - pending.length} />

        <div className="flex items-center gap-2">
          <button
            type="button"
            disabled={items.length === 0}
            onClick={() =>
              void run(
                items.map((item) => item.id),
                async () => {
                  const result = await clearAllTrash();
                  return result.ok
                    ? { ok: true, message: `已彻底删除 ${result.count} 部剧目` }
                    : result;
                },
                "回收站已清空",
              )
            }
            className="flex items-center gap-1.5 rounded-full border border-red-400/25 px-3.5 py-2 text-[12.5px] text-red-400 transition-colors hover:bg-white/5 disabled:cursor-not-allowed disabled:opacity-40"
          >
            <Eraser className="h-3.5 w-3.5" />
            清空回收站
          </button>

          <div className="relative">
            <Search className="pointer-events-none absolute left-3.5 top-2.5 h-4 w-4 text-white/30" />
            <input
              type="text"
              value={keyword}
              onChange={(e) => setKeyword(e.target.value)}
              placeholder="搜索回收站"
              className="w-[164px] rounded-full border border-hairline bg-ink-800 py-2 pl-9 pr-3 text-[12.5px] text-white placeholder:text-white/30 focus:border-accent/60 focus:outline-none xl:w-[220px]"
            />
          </div>
        </div>
      </div>

      <TrashView
        items={visible}
        menuId={menuId}
        onToggleMenu={(id) => setMenuId((current) => (current === id ? null : id))}
        onAction={handleAction}
      />
    </main>
  );
}
