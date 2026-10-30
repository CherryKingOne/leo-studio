"use client";

import { Bell, Plus } from "lucide-react";
import { useImportNovelModal } from "@/components/import-novel-modal";

const pill =
  "flex items-center gap-2 rounded-full border border-hairline bg-ink-800 text-[12.5px] text-white/70 transition-colors hover:border-white/20";

export function TopBar() {
  const { open: openImportModal } = useImportNovelModal();

  return (
    <header className="sticky top-0 z-30 flex h-16 shrink-0 items-center justify-between gap-3 bg-ink-900/85 px-5 backdrop-blur-md sm:px-8">
      <button
        type="button"
        onClick={openImportModal}
        className="flex items-center gap-2 rounded-lg border border-hairline bg-ink-800 px-3 py-2 text-[13px] text-white/70 transition-colors hover:bg-ink-700 lg:hidden"
      >
        <Plus className="h-4 w-4" />
        新建
      </button>

      <div className="hidden lg:block" />

      <div className="flex items-center gap-2">
        <button type="button" className={`${pill} hidden py-1.5 pl-1.5 pr-3.5 xl:flex`}>
          <span className="flex h-6 w-6 items-center justify-center rounded-full bg-accent/15 text-[11px] font-bold text-accent-400">
            新
          </span>
          拆本引擎 v3：EPUB 章节自动识别
        </button>

        <button
          type="button"
          title="通知"
          className="relative rounded-lg p-2 text-white/50 transition-colors hover:bg-white/5 hover:text-white"
        >
          <Bell className="h-[18px] w-[18px]" />
          <span className="absolute -right-1 -top-1 flex h-4 min-w-[17px] items-center justify-center rounded-full bg-accent px-1 text-[9px] font-bold text-white">
            12
          </span>
        </button>

        <button
          type="button"
          className="ml-1 h-8 w-8 shrink-0 rounded-full bg-ink-700 text-[12px] font-bold text-white/70 ring-1 ring-white/10 transition hover:ring-white/25"
        >
          L
        </button>
      </div>
    </header>
  );
}
