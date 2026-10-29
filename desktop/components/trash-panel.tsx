"use client";

import { Search } from "lucide-react";
import { useState } from "react";
import { LibraryTabs } from "@/components/library-tabs";
import { TrashView } from "@/components/trash-view";
import { purgeProject, restoreProjects, useTrash } from "@/lib/trash-store";

export function TrashPanel() {
  const [keyword, setKeyword] = useState("");
  const items = useTrash();

  const trashed = items.filter((item) => !item.restored);
  const needle = keyword.trim().toLowerCase();
  const visible = trashed.filter(
    (item) => !needle || item.project.title.toLowerCase().includes(needle),
  );

  function restore(id: string) {
    restoreProjects(new Set([id]));
  }

  function purge(id: string) {
    purgeProject(id);
  }

  return (
    <main className="flex-1 px-5 pb-16 pt-8 sm:px-8">
      <h1 className="text-[26px] font-bold leading-none tracking-tight text-white">我的剧目</h1>

      <div className="mt-6 flex flex-wrap items-center justify-between gap-3">
        <LibraryTabs trashCount={trashed.length} />

        <div className="flex items-center gap-2">
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

      <TrashView items={visible} onRestore={restore} onPurge={purge} />
    </main>
  );
}
