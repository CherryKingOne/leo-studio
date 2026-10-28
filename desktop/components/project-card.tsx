import {
  ArrowUpRight,
  FolderX,
  MoreHorizontal,
  Pencil,
  RotateCcw,
  Star,
  Trash2,
  type LucideIcon,
} from "lucide-react";
import Link from "next/link";
import { Waveform } from "@/components/waveform";
import {
  coverHint,
  projectMeta,
  type GenreTone,
  type ProjectEntry,
} from "@/lib/projects";

export type CardAction = "open" | "rename" | "fav" | "trash" | "restore" | "purge";

const toneDot: Record<GenreTone, string> = {
  accent: "bg-accent",
  emerald: "bg-emerald-400",
  amber: "bg-amber-400",
};

const menuRow =
  "flex w-full items-center gap-2.5 px-3 py-2 text-left text-[12.5px] text-white/70 hover:bg-white/5 hover:text-white";

export function ProjectCard({
  project,
  menuOpen,
  renaming,
  onToggleMenu,
  onAction,
  onRename,
}: {
  project: ProjectEntry;
  menuOpen: boolean;
  renaming: boolean;
  onToggleMenu: (id: string) => void;
  onAction: (id: string, action: CardAction) => void;
  onRename: (id: string, title: string) => void;
}) {
  const rows: [CardAction, LucideIcon, string][] = project.trashed
    ? [
        ["restore", RotateCcw, "恢复"],
        ["purge", Trash2, "彻底删除"],
      ]
    : [
        ["open", ArrowUpRight, "打开"],
        ["rename", Pencil, "重命名"],
        ["fav", Star, project.fav ? "取消收藏" : "收藏"],
        ["trash", FolderX, "移入回收站"],
      ];

  return (
    <article className="relative rounded-2xl border border-hairline bg-ink-800 transition-colors hover:border-white/16">
      <ProjectCover project={project} />

      <div className="flex items-end justify-between gap-2 border-t border-hairline px-4 py-3">
        <div className="min-w-0">
          {renaming ? (
            <input
              defaultValue={project.title}
              autoFocus
              onBlur={(e) => onRename(project.id, e.target.value.trim())}
              onKeyDown={(e) => {
                if (e.key === "Enter") e.currentTarget.blur();
                if (e.key === "Escape") onRename(project.id, project.title);
              }}
              className="w-full truncate rounded-md bg-white/5 px-1.5 py-0.5 text-[14px] font-medium text-white outline-none ring-1 ring-accent/60"
            />
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
          className="shrink-0 rounded-md p-1 text-white/30 transition-colors hover:bg-white/5 hover:text-white/70"
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
              className={menuRow}
            >
              <Icon className="h-3.5 w-3.5 opacity-60" />
              {label}
            </button>
          ))}
        </div>
      )}
    </article>
  );
}

function ProjectCover({ project }: { project: ProjectEntry }) {
  const complete = project.progress === 100;

  return (
    <Link
      href={`/project/${project.id}`}
      className="group relative block aspect-[16/10] overflow-hidden rounded-t-2xl bg-card-cover"
    >
      <span className="absolute left-4 top-4 flex items-center gap-1.5 rounded-full bg-black/45 px-2.5 py-1 text-[10.5px] font-medium text-white/75 ring-1 ring-white/10">
        <span className={`h-1.5 w-1.5 rounded-full ${toneDot[project.tone]}`} />
        {project.genre}
      </span>

      {complete && (
        <span className="absolute right-4 top-4 rounded-full bg-emerald-400/10 px-2.5 py-1 text-[10.5px] font-medium text-emerald-300 ring-1 ring-emerald-400/20">
          可导出 M4B
        </span>
      )}

      <Waveform
        progress={project.progress}
        seed={project.seed}
        className="absolute inset-x-0 top-1/2 h-16 w-full -translate-y-1/2 px-5"
      />

      <div className="absolute inset-x-4 bottom-4">
        <div className="mb-1.5 flex items-center justify-between text-[10.5px] text-white/35">
          <span>{coverHint(project)}</span>
          <span className={`font-mono ${complete ? "text-emerald-400/80" : ""}`}>
            {project.progress}%
          </span>
        </div>
        <div className="h-[3px] w-full overflow-hidden rounded-full bg-white/10">
          <div
            className="h-full rounded-full bg-accent"
            style={{ width: `${project.progress}%` }}
          />
        </div>
      </div>
    </Link>
  );
}
