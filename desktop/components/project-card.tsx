import {
  ArrowUpRight,
  Check,
  FileAudio,
  ListChecks,
  MoreHorizontal,
  Pencil,
  Trash2,
  type LucideIcon,
} from "lucide-react";
import Link from "next/link";
import { projectMeta, type ProjectSummary } from "@/lib/projects";

export type CardAction = "open" | "rename" | "delete" | "batch";

const menuRow =
  "flex w-full items-center gap-2.5 px-3 py-2 text-left text-[12.5px] text-white transition-colors hover:bg-white/5";

const menuRowDanger =
  "flex w-full items-center gap-2.5 px-3 py-2 text-left text-[12.5px] text-red-400 transition-colors hover:bg-white/5";

export function ProjectCard({
  project,
  menuOpen,
  renaming,
  batchMode = false,
  selected = false,
  onToggleMenu,
  onAction,
  onRename,
  onToggleSelect,
}: {
  project: ProjectSummary;
  menuOpen: boolean;
  renaming: boolean;
  /** 批量管理态：封面点击改为勾选 */
  batchMode?: boolean;
  selected?: boolean;
  onToggleMenu: (id: string) => void;
  onAction: (id: string, action: CardAction) => void;
  onRename: (id: string, title: string) => void;
  onToggleSelect?: (id: string) => void;
}) {
  const rows: [CardAction, LucideIcon, string][] = [
    ["open", ArrowUpRight, "打开"],
    ["rename", Pencil, "重命名"],
    ["delete", Trash2, "删除"],
    ["batch", ListChecks, "批量管理"],
  ];

  const cover = (
    <>
      <FileAudio className="absolute left-1/2 top-1/2 h-9 w-9 -translate-x-1/2 -translate-y-1/2 text-white/12" />

      {batchMode && (
        <span
          aria-hidden
          className={`absolute right-3 top-3 flex size-5 items-center justify-center rounded-md border transition-colors ${
            selected
              ? "border-accent bg-accent text-white"
              : "border-white/35 bg-black/45 text-transparent backdrop-blur-sm"
          }`}
        >
          <Check className="h-3 w-3 strokeWidth={3}" />
        </span>
      )}

      <div className="absolute inset-x-4 bottom-4 truncate font-mono text-[10.5px] text-white/30">
        {project.filename}
      </div>
    </>
  );

  return (
    <article
      className={`relative rounded-2xl border bg-ink-800 transition-colors ${
        selected ? "border-accent/60 ring-2 ring-accent/35" : "border-hairline hover:border-white/16"
      }`}
    >
      <div className="relative block aspect-[16/10] overflow-hidden rounded-t-2xl bg-card-cover">
        {batchMode ? (
          <button
            type="button"
            aria-pressed={selected}
            aria-label={`${selected ? "取消选择" : "选择"}${project.title}`}
            onClick={() => onToggleSelect?.(project.id)}
            className="absolute inset-0 cursor-pointer"
          >
            {cover}
          </button>
        ) : (
          <Link href={`/project/${project.id}`} className="group absolute inset-0">
            {cover}
          </Link>
        )}
      </div>

      <div className="flex items-end justify-between gap-2 border-t border-hairline px-4 py-3">
        <div className="min-w-0">
          {renaming ? (
            <div className="flex items-center gap-1.5 rounded-md bg-white/5 px-1.5 ring-1 ring-accent/60">
              <Pencil className="h-3 w-3 shrink-0 text-accent-400" />
              <input
                defaultValue={project.title}
                autoFocus
                onBlur={(e) => onRename(project.id, e.target.value.trim())}
                onKeyDown={(e) => {
                  if (e.key === "Enter") e.currentTarget.blur();
                  if (e.key === "Escape") onRename(project.id, project.title);
                }}
                className="w-full truncate bg-transparent py-1 text-[14px] font-medium text-white outline-none"
              />
            </div>
          ) : (
            <h3 className="truncate text-[14px] font-medium text-white">{project.title}</h3>
          )}
          <p className="mt-1 truncate text-[11.5px] text-white/35">{projectMeta(project)}</p>
        </div>

        <button
          type="button"
          data-menu
          aria-label={`${project.title} 操作菜单`}
          onClick={() => onToggleMenu(project.id)}
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
              onClick={() => onAction(project.id, action)}
              className={action === "delete" ? menuRowDanger : menuRow}
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
