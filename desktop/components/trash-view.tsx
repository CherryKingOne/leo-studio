"use client";

import { FileAudio, Inbox, Trash2, Undo2 } from "lucide-react";
import { projectMeta, type ProjectSummary } from "@/lib/projects";

export type TrashedProject = {
  project: ProjectSummary;
  /** 删除时刻（客户端时间戳），回收站只在本次会话内存在 */
  deletedAt: number;
};

const actionButton =
  "flex items-center gap-1.5 rounded-lg border border-hairline bg-ink-850 px-3 py-1.5 text-[12.5px] text-white/70 transition-colors hover:text-white";

function TrashCard({
  item,
  onRestore,
  onPurge,
}: {
  item: TrashedProject;
  onRestore: (id: string) => void;
  onPurge: (id: string) => void;
}) {
  const { project } = item;

  return (
    <article className="relative rounded-2xl border border-hairline bg-ink-800 transition-colors hover:border-white/16">
      <div className="relative block aspect-[16/10] overflow-hidden rounded-t-2xl bg-card-cover opacity-60">
        <FileAudio className="absolute left-1/2 top-1/2 h-9 w-9 -translate-x-1/2 -translate-y-1/2 text-white/12" />
        <div className="absolute inset-x-4 bottom-4 truncate font-mono text-[10.5px] text-white/30">
          {project.filename}
        </div>
      </div>

      <div className="border-t border-hairline px-4 py-3">
        <h3 className="truncate text-[14px] font-medium text-white/70">{project.title}</h3>
        <p className="mt-1 truncate text-[11.5px] text-white/35">{projectMeta(project)}</p>

        <div className="mt-3 flex items-center gap-2">
          <button
            type="button"
            onClick={() => onRestore(project.id)}
            className={`${actionButton} border-accent/25 bg-accent/10 text-accent-400 hover:text-accent-400`}
          >
            <Undo2 className="h-3.5 w-3.5" />
            恢复
          </button>
          <button
            type="button"
            onClick={() => onPurge(project.id)}
            className={`${actionButton} text-red-400 hover:bg-red-500/10`}
          >
            <Trash2 className="h-3.5 w-3.5" />
            彻底删除
          </button>
        </div>
      </div>
    </article>
  );
}

export function TrashView({
  items,
  onRestore,
  onPurge,
}: {
  items: TrashedProject[];
  onRestore: (id: string) => void;
  onPurge: (id: string) => void;
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
          key={item.project.id}
          item={item}
          onRestore={onRestore}
          onPurge={onPurge}
        />
      ))}
    </div>
  );
}
