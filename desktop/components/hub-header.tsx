"use client";

import { ChevronLeft, Download, Pencil } from "lucide-react";
import Link from "next/link";
import { useState } from "react";
import { useHubChapters } from "@/components/hub-chapters";
import { useToast } from "@/components/toast";
import { renameProjectAction } from "@/lib/actions";
import type { HubIdentity } from "@/lib/projects";

export function HubHeader({
  projectId,
  identity,
  remote,
}: {
  projectId: string;
  identity: HubIdentity;
  /** 后端剧目才改名；原型剧目没有 project_id 对应记录，只在本地改标题 */
  remote: boolean;
}) {
  const { show } = useToast();
  const { chapters } = useHubChapters();
  const [title, setTitle] = useState(identity.title);
  const [editing, setEditing] = useState(false);

  async function commit(value: string) {
    setEditing(false);
    const next = value.trim();
    if (!next || next === title) return;

    const previous = title;
    setTitle(next);
    if (!remote) {
      show("剧目名称已更新", "success");
      return;
    }

    // PATCH /api/v1/projects/{id} 成功后服务端动作会 refresh()，列表与侧栏同步换新名
    const result = await renameProjectAction(projectId, next);
    if (result.ok) show("剧目名称已更新", "success");
    else {
      setTitle(previous);
      show(result.message, "info");
    }
  }

  return (
    <header className="sticky top-0 z-30 bg-ink-900/85 backdrop-blur-md">
      <div className="mx-auto flex h-16 w-full max-w-6xl items-center gap-3 px-5 sm:gap-4 sm:px-8">
        <Link
          href="/project"
          title="返回书架"
          className="flex h-8 shrink-0 items-center gap-1 rounded-full border border-hairline bg-ink-800 px-2 text-[12.5px] text-white/55 transition-colors hover:border-white/20 hover:text-white"
        >
          <ChevronLeft className="h-4 w-4" />
          <span className="hidden sm:inline">书架</span>
        </Link>

        {/* 迷你封面：与剧目库卡片的封面同一套底色，看板与列表之间靠它认剧 */}
        <div
          aria-hidden
          className="flex h-9 w-9 shrink-0 items-center justify-center rounded-[10px] border border-hairline bg-card-cover text-[14px] font-semibold text-white/75"
        >
          {title.slice(0, 1)}
        </div>

        <div className="group min-w-0 flex-1">
          <div className="flex min-w-0 items-center gap-2">
            {editing ? (
              <input
                defaultValue={title}
                autoFocus
                onBlur={(e) => commit(e.target.value.trim())}
                onKeyDown={(e) => {
                  if (e.key === "Enter") e.currentTarget.blur();
                  if (e.key === "Escape") setEditing(false);
                }}
                className="w-56 min-w-0 truncate rounded-md bg-white/5 px-1.5 py-0.5 text-[15px] font-semibold tracking-tight text-white outline-none ring-1 ring-accent/60"
              />
            ) : (
              <h1
                title="双击重命名剧目"
                onDoubleClick={() => setEditing(true)}
                className="truncate text-[15px] font-semibold leading-tight tracking-tight text-white"
              >
                {title}
              </h1>
            )}

            {!editing && (
              <button
                type="button"
                title="重命名剧目"
                onClick={() => setEditing(true)}
                className="shrink-0 rounded p-1 text-white/30 opacity-0 transition-opacity hover:bg-white/5 hover:text-white focus-visible:opacity-100 group-hover:opacity-100"
              >
                <Pencil className="h-3.5 w-3.5" />
              </button>
            )}
          </div>
          <div className="mt-1 truncate text-[11.5px] leading-tight text-white/35">
            {/* 章节数不另起胶囊：紧贴下方的 tab 已带同一份计数，再画一个框只会增加轮廓 */}
            {chapters.length} 章 · {identity.subtitle}
          </div>
        </div>

        <div className="flex shrink-0 items-center gap-3 sm:gap-4">
          {identity.progress !== undefined && (
            <div className="hidden items-center gap-2 text-[12px] text-white/40 lg:flex">
              <span>全书进度</span>
              <div className="h-[3px] w-20 overflow-hidden rounded-full bg-white/10">
                <div
                  className="h-full rounded-full bg-accent"
                  style={{ width: `${identity.progress}%` }}
                />
              </div>
              <span className="font-mono text-white/60">{identity.progress}%</span>
            </div>
          )}

          <Link
            href={`/project/${projectId}/export`}
            className="flex h-8 shrink-0 items-center gap-2 rounded-full border border-hairline bg-ink-800 px-3 text-[12.5px] font-medium text-white/75 transition-colors hover:border-emerald-400/30 hover:bg-emerald-400/10 hover:text-emerald-300"
          >
            <Download className="h-4 w-4 text-emerald-400/80" />
            <span>导出音频</span>
          </Link>
        </div>
      </div>
    </header>
  );
}
