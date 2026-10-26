"use client";

import { BookOpen, ChevronLeft, Download } from "lucide-react";
import Link from "next/link";
import { useState } from "react";
import { useToast } from "@/components/toast";
import { hubSubtitle, type HubChapter, type ProjectEntry } from "@/lib/projects";

export function HubHeader({
  project,
  chapters,
}: {
  project: ProjectEntry;
  chapters: HubChapter[];
}) {
  const { show } = useToast();
  const [title, setTitle] = useState(project.title);
  const [editing, setEditing] = useState(false);

  function commit(value: string) {
    setEditing(false);
    if (!value || value === title) return;
    setTitle(value);
    show("剧目名称已更新", "success");
  }

  return (
    <header className="sticky top-0 z-30 flex h-16 shrink-0 items-center justify-between gap-3 border-b border-hairline bg-ink-900/85 px-5 backdrop-blur-md sm:px-8">
      <div className="flex min-w-0 items-center gap-4">
        <Link
          href="/project"
          className="flex shrink-0 items-center gap-1.5 rounded-lg border border-hairline bg-ink-800 px-2.5 py-1.5 text-[12.5px] text-white/60 transition-colors hover:bg-ink-700 hover:text-white"
        >
          <ChevronLeft className="h-4 w-4" />
          <span>返回书架</span>
        </Link>

        <div className="h-4 w-px shrink-0 bg-white/7" />

        <div className="flex min-w-0 items-center gap-3">
          <div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-xl border border-hairline bg-ink-800 text-white/50">
            <BookOpen className="h-[18px] w-[18px]" />
          </div>
          <div className="min-w-0">
            <div className="flex items-center gap-2">
              {editing ? (
                <input
                  defaultValue={title}
                  autoFocus
                  onBlur={(e) => commit(e.target.value.trim())}
                  onKeyDown={(e) => {
                    if (e.key === "Enter") e.currentTarget.blur();
                    if (e.key === "Escape") setEditing(false);
                  }}
                  className="min-w-0 truncate rounded-md bg-white/5 px-1.5 text-[15px] font-semibold leading-none tracking-tight text-white outline-none ring-1 ring-accent/60"
                />
              ) : (
                <h1
                  title="双击重命名剧目"
                  onDoubleClick={() => setEditing(true)}
                  className="truncate text-[15px] font-semibold leading-none tracking-tight text-white"
                >
                  {title}
                </h1>
              )}
              <span className="shrink-0 rounded-full border border-hairline bg-ink-800 px-2 py-0.5 font-mono text-[10.5px] text-white/50">
                {project.chapters} 章节
              </span>
            </div>
            <div className="mt-1 truncate text-[11.5px] text-white/35">
              {hubSubtitle(project, chapters)}
            </div>
          </div>
        </div>
      </div>

      <div className="flex shrink-0 items-center gap-3">
        <div className="hidden items-center gap-2 text-[12px] text-white/45 lg:flex">
          <span>全书进度</span>
          <div className="h-[3px] w-24 overflow-hidden rounded-full bg-white/10">
            <div
              className="h-full rounded-full bg-accent"
              style={{ width: `${project.progress}%` }}
            />
          </div>
          <span className="font-mono text-white/60">{project.progress}%</span>
        </div>

        <Link
          href={`/project/${project.id}/export`}
          className="flex items-center gap-2 rounded-lg border border-hairline bg-ink-800 px-3.5 py-2 text-[12.5px] font-medium text-white/75 transition-colors hover:border-emerald-400/30 hover:bg-emerald-400/10 hover:text-emerald-300"
        >
          <Download className="h-4 w-4 text-emerald-400/80" />
          <span>导出音频</span>
        </Link>
      </div>
    </header>
  );
}
