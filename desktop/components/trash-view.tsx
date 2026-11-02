"use client";

import { FileAudio, Inbox, MoreHorizontal, Trash2, Undo2, type LucideIcon } from "lucide-react";
import { projectMeta, type TrashedProject } from "@/lib/projects";

const menuRow =
  "flex w-full items-center gap-2.5 px-3 py-2 text-left text-[12.5px] text-white transition-colors hover:bg-white/5";

const menuDanger =
  "flex w-full items-center gap-2.5 px-3 py-2 text-left text-[12.5px] text-red-400 transition-colors hover:bg-white/5";

const rows: ["restore" | "purge", LucideIcon, string][] = [
  ["restore", Undo2, "恢复"],
  ["purge", Trash2, "彻底删除"],
];

function TrashCard({
  item,
  menuOpen,
  onToggleMenu,
  onAction,
}: {
  item: TrashedProject;
  menuOpen: boolean;
  onToggleMenu: (id: string) => void;
  onAction: (id: string, action: "restore" | "purge") => void;
}) {
  return (
    <article className="relative rounded-2xl border border-hairline bg-ink-800 transition-colors hover:border-white/16">
      <div className="relative block aspect-[16/10] overflow-hidden rounded-t-2xl bg-card-cover opacity-60">
        <FileAudio className="absolute left-1/2 top-1/2 h-9 w-9 -translate-x-1/2 -translate-y-1/2 text-white/12" />
        <div className="absolute inset-x-4 bottom-4 truncate font-mono text-[10.5px] text-white/30">
          {item.filename}
        </div>
      </div>

      <div className="flex items-end justify-between gap-2 border-t border-hairline px-4 py-3">
        <div className="min-w-0">
          <h3 className="truncate text-[14px] font-medium text-white/70">{item.title}</h3>
          <p className="mt-1 truncate text-[11.5px] text-white/35">{projectMeta(item)}</p>
        </div>
        <button
          type="button"
          data-menu
          aria-label={`${item.title} 操作菜单`}
          onClick={() => onToggleMenu(item.id)}
          className="shrink-0 rounded-md p-1 text-white/70 transition-colors hover:bg-white/5 hover:text-white"
        >
          <MoreHorizontal className="h-4 w-4" />
        </button>
      </div>

      {menuOpen && (
        <div
          data-menu
          className="absolute bottom-[54px] right-3 z-20 w-36 overflow-hidden rounded-xl border border-hairline bg-ink-750 py-1 shadow-2xl shadow-black/60"
        >
          {rows.map(([action, Icon, label]) => (
            <button
              key={action}
              type="button"
              onClick={() => onAction(item.id, action)}
              className={action === "purge" ? menuDanger : menuRow}
            >
              <Icon className="h-3.5 w-3.5" />
              {label}
            </button>
          ))}
        </div>
      )}
    </article>
  );
}

export function TrashView({
  items,
  menuId,
  onToggleMenu,
  onAction,
}: {
  items: TrashedProject[];
  menuId: string | null;
  onToggleMenu: (id: string) => void;
  onAction: (id: string, action: "restore" | "purge") => void;
}) {
  if (items.length === 0) {
    return (
      <div className="flex min-h-[420px] flex-col items-center justify-center text-center">
        <Inbox className="h-12 w-12 text-white/15" />
        <p className="mt-4 text-[13px] text-white/35">回收站为空</p>
      </div>
    );
  }

  return (
    <div className="mt-6 grid grid-cols-1 gap-5 sm:grid-cols-2 xl:grid-cols-3 2xl:grid-cols-4">
      {items.map((item) => (
        <TrashCard
          key={item.id}
          item={item}
          menuOpen={menuId === item.id}
          onToggleMenu={onToggleMenu}
          onAction={onAction}
        />
      ))}
    </div>
  );
}
