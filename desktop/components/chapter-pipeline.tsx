"use client";

import {
  AlignLeft,
  BadgeCheck,
  Check,
  ChevronDown,
  FileCheck2,
  FilePlus2,
  Hourglass,
  Loader,
  Play,
  ScanSearch,
  SlidersHorizontal,
  UserRoundSearch,
  WandSparkles,
  type LucideIcon,
} from "lucide-react";
import Link from "next/link";
import { useMemo, useState } from "react";
import { useHubChapters } from "@/components/hub-chapters";
import { useToast } from "@/components/toast";
import {
  chapterIsSelectable,
  formatClock,
  formatWords,
  pipelineFilters,
  type ChapterStatusIcon,
  type HubChapter,
  type HubTone,
} from "@/lib/projects";

const statusIcon: Record<ChapterStatusIcon, LucideIcon> = {
  master: BadgeCheck,
  script: FileCheck2,
  queue: AlignLeft,
  scan: ScanSearch,
  cast: UserRoundSearch,
};

const pillTone: Record<HubTone, string> = {
  neutral: "border-white/10 bg-white/5 text-white/45",
  muted: "border-white/10 bg-white/5 text-white/45",
  accent: "border-accent/25 bg-accent/10 text-accent-400",
  emerald: "border-emerald-400/20 bg-emerald-400/10 text-emerald-300",
  amber: "border-amber-400/20 bg-amber-400/10 text-amber-300",
  violet: "border-violet-400/20 bg-violet-400/10 text-violet-300",
};

const speakerTone: Record<HubTone, string> = {
  neutral: "border-white/10 bg-white/5 text-white/60",
  muted: "border-white/10 bg-white/5 text-white/45",
  accent: "border-accent/25 bg-accent/10 text-accent-400",
  emerald: "border-emerald-400/20 bg-emerald-400/10 text-emerald-300",
  amber: "border-amber-400/20 bg-amber-400/10 text-amber-300",
  violet: "border-violet-400/20 bg-violet-400/10 text-violet-300",
};

const badgeTone: Record<HubTone, string> = {
  neutral: "border-hairline bg-ink-850 text-white/40",
  muted: "border-hairline bg-ink-850 text-white/35",
  accent: "border-accent/25 bg-accent/10 text-accent-400",
  emerald: "border-emerald-400/20 bg-emerald-400/10 text-emerald-400/90",
  amber: "border-amber-400/20 bg-amber-400/10 text-amber-300",
  violet: "border-violet-400/20 bg-violet-400/10 text-violet-300",
};

const rowTint: Partial<Record<HubTone, string>> = {
  accent: "bg-accent/[0.03]",
  amber: "bg-amber-400/[0.03]",
};

const toolbarButton =
  "flex items-center gap-1.5 rounded-full border border-hairline bg-ink-800 px-3.5 py-2 text-[12.5px] text-white/70 transition-colors hover:border-white/20 hover:text-white disabled:cursor-not-allowed disabled:opacity-40";

function Tick({
  checked,
  disabled,
  title,
  onChange,
}: {
  checked: boolean;
  disabled?: boolean;
  title?: string;
  onChange?: (checked: boolean) => void;
}) {
  return (
    <button
      type="button"
      role="checkbox"
      aria-checked={checked}
      disabled={disabled}
      title={title}
      onClick={() => onChange?.(!checked)}
      className={`flex size-[15px] shrink-0 items-center justify-center rounded transition-colors ${
        checked
          ? "border border-accent bg-accent text-white"
          : "border border-white/18 bg-ink-800 hover:border-white/30"
      } ${disabled ? "cursor-not-allowed opacity-40 hover:border-white/18" : "cursor-pointer"}`}
    >
      {checked && <Check className="h-2.5 w-2.5 strokeWidth={3}" />}
    </button>
  );
}

export function ChapterPipeline({
  projectId,
  appendReady,
  keyword,
  readyOnly,
  onGoToCast,
}: {
  projectId: string;
  /** 只有后端剧目能追更上传章节 */
  appendReady: boolean;
  keyword: string;
  readyOnly: boolean;
  onGoToCast: () => void;
}) {
  const { show } = useToast();
  const { chapters, filter, setFilter, splittingIds, split, openImport } = useHubChapters();
  const [expanded, setExpanded] = useState(false);
  const [selected, setSelected] = useState<string[]>([]);

  const visible = useMemo(() => {
    const needle = keyword.trim().toLowerCase();
    return chapters.filter((chapter) => {
      const speakers = chapter.speakers?.map((tag) => tag.name).join(" ") ?? "";
      return (
        (expanded || !chapter.collapsed) &&
        (filter === "全部" || chapter.state === filter) &&
        (!readyOnly || chapter.state === "已就绪") &&
        (!needle || `${chapter.title} ${speakers}`.toLowerCase().includes(needle))
      );
    });
  }, [chapters, filter, readyOnly, keyword, expanded]);

  const selectable = visible.filter(chapterIsSelectable);
  const allSelected =
    selectable.length > 0 && selectable.every((chapter) => selected.includes(chapter.id));
  const collapsedCount = chapters.filter((chapter) => chapter.collapsed).length;
  const unparsedCount = chapters.filter((chapter) => chapter.state === "待拆解").length;

  function toggle(chapter: HubChapter, checked: boolean) {
    setSelected((prev) =>
      checked ? [...prev, chapter.id] : prev.filter((id) => id !== chapter.id),
    );
  }

  function toggleAll(checked: boolean) {
    const ids = selectable.map((chapter) => chapter.id);
    setSelected((prev) =>
      checked
        ? [...new Set([...prev, ...ids])]
        : prev.filter((id) => !ids.includes(id)),
    );
  }

  function synthesize(chapter: HubChapter) {
    show(`《${chapter.title}》已进入合成队列，预计 4 分钟`, "info");
  }

  function batchSynthesize() {
    show(`已把 ${selected.length} 章加入合成队列`, "success");
    setSelected([]);
  }

  return (
    <div className="mt-5">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div className="flex flex-wrap items-center gap-1.5">
          {pipelineFilters.map(({ state, label }) => {
            const on = filter === state;
            return (
              <button
                key={state}
                type="button"
                onClick={() => setFilter(state)}
                className={`rounded-full px-3 py-1.5 text-[12px] transition-colors ${
                  on
                    ? "border border-white/12 bg-white/7 text-white"
                    : "border border-hairline bg-ink-800 text-white/50 hover:text-white"
                }`}
              >
                {label}
              </button>
            );
          })}
        </div>

        <div className="flex items-center gap-2">
          {selected.length > 0 && (
            <span className="font-mono text-[11.5px] text-white/40">
              已选 {selected.length} 章
            </span>
          )}
          <button
            type="button"
            disabled={selected.length === 0}
            onClick={batchSynthesize}
            className={toolbarButton}
          >
            <Play className="h-3.5 w-3.5" />
            批量合成所选
          </button>
          <button
            type="button"
            onClick={() => show(`已把 ${unparsedCount} 章加入拆本队列，预计 6 分钟完成`, "info")}
            className={toolbarButton}
          >
            <WandSparkles className="h-3.5 w-3.5" />
            一键拆解全书
          </button>
          {appendReady && (
            <button
              type="button"
              onClick={openImport}
              className="flex items-center gap-1.5 rounded-full bg-accent px-3.5 py-2 text-[12.5px] font-medium text-white transition-colors hover:bg-accent-600"
            >
              <FilePlus2 className="h-3.5 w-3.5" />
              追更上传章节
            </button>
          )}
        </div>
      </div>

      <div className="mt-4 overflow-x-auto rounded-2xl border border-hairline bg-ink-800">
        <table className="w-full min-w-[920px] text-left text-[12.5px]">
          <thead className="border-b border-hairline bg-ink-850/80 font-mono text-[10.5px] uppercase text-white/35">
            <tr>
              <th className="w-11 py-3 pl-4 pr-1">
                <Tick
                  checked={allSelected}
                  disabled={selectable.length === 0}
                  title="全选可合成章节"
                  onChange={toggleAll}
                />
              </th>
              <th className="px-4 py-3 font-medium">章节序号与标题</th>
              <th className="px-4 py-3 font-medium">字数与时长</th>
              <th className="px-4 py-3 font-medium">对白角色识别</th>
              <th className="px-4 py-3 font-medium">生产流水线状态</th>
              <th className="px-6 py-3 text-right font-medium">操作</th>
            </tr>
          </thead>
          <tbody className="divide-y divide-white/6">
            {visible.map((chapter) => {
              const studioHref = `/project/${projectId}/chapter/${chapter.id}`;
              const Icon = chapter.status ? statusIcon[chapter.status.icon] : null;
              return (
                <tr
                  key={chapter.id}
                  className={`group transition-colors hover:bg-white/3 ${rowTint[chapter.tone] ?? ""}`}
                >
                  <td className="py-4 pl-4 pr-1">
                    {chapterIsSelectable(chapter) ? (
                      <Tick
                        checked={selected.includes(chapter.id)}
                        onChange={(checked) => toggle(chapter, checked)}
                      />
                    ) : (
                      <Tick
                        checked={false}
                        disabled
                        title={chapter.state === "已就绪" ? "已完成，无需再生成" : undefined}
                      />
                    )}
                  </td>
                  <td className="px-4 py-4">
                    <div className="flex items-center gap-3">
                      <div
                        className={`flex h-7 w-7 shrink-0 items-center justify-center rounded-lg font-mono text-[11px] font-bold ${badgeTone[chapter.tone]}`}
                      >
                        {chapter.no}
                      </div>
                      <div className="min-w-0">
                        <div className="flex min-w-0 items-center gap-2">
                          {chapter.openable ? (
                            <Link
                              href={studioHref}
                              className="truncate text-[13.5px] font-medium text-white transition-colors group-hover:text-accent-400"
                            >
                              {chapter.title}
                            </Link>
                          ) : (
                            <span className="truncate text-[13.5px] font-medium text-white/60">
                              {chapter.title}
                            </span>
                          )}
                        </div>
                        <div
                          className={`mt-0.5 truncate text-[11px] ${
                            !chapter.openable
                              ? "text-white/25"
                              : chapter.tone === "amber"
                                ? "text-amber-300/60"
                                : "text-white/35"
                          }`}
                        >
                          {chapter.excerpt}
                        </div>
                      </div>
                    </div>
                  </td>
                  <td className="px-4 py-4 font-mono">
                    <div className={chapter.openable ? "text-white/75" : "text-white/50"}>
                      {chapter.words ? `${formatWords(chapter.words)} 字` : "待统计"}
                    </div>
                    <div
                      className={`mt-0.5 text-[11px] ${
                        chapter.masterSeconds
                          ? "text-emerald-400/80"
                          : chapter.openable
                            ? "text-white/30"
                            : "text-white/25"
                      }`}
                    >
                      {chapter.masterSeconds
                        ? `${formatClock(chapter.masterSeconds)} (已渲染)`
                        : chapter.etaSeconds
                          ? `预估 ${formatClock(chapter.etaSeconds)}`
                          : "未合成"}
                    </div>
                  </td>
                  <td className="px-4 py-4">
                    {chapter.speakers ? (
                      <div className="flex flex-wrap items-center gap-1.5">
                        {chapter.speakers.map((tag) => (
                          <span
                            key={tag.name}
                            className={`rounded border px-1.5 py-0.5 text-[10px] ${speakerTone[tag.tone]}`}
                          >
                            {tag.name}
                          </span>
                        ))}
                      </div>
                    ) : (
                      <div className="text-[11px] text-white/30">待解析</div>
                    )}
                  </td>
                  <td className="px-4 py-4">
                    {chapter.percent !== undefined && Icon === null ? (
                      <div>
                        <div className="flex items-center justify-between gap-3 font-mono text-[10.5px] text-white/40">
                          <span className="flex items-center gap-1.5 text-accent-400">
                            <span className="h-1.5 w-1.5 animate-pulse rounded-full bg-accent" />
                            合成中
                          </span>
                          <span>{chapter.percent}%</span>
                        </div>
                        <div className="mt-1.5 h-[3px] w-28 overflow-hidden rounded-full bg-white/10">
                          <div
                            className="h-full rounded-full bg-accent transition-all duration-700"
                            style={{ width: `${chapter.percent}%` }}
                          />
                        </div>
                      </div>
                    ) : (
                      chapter.status && (
                        <span
                          className={`inline-flex items-center gap-1.5 rounded-full border px-2.5 py-1 text-[11px] font-medium ${pillTone[chapter.status.tone]}`}
                        >
                          {Icon && <Icon className="h-3.5 w-3.5" />}
                          <span>{chapter.status.label}</span>
                        </span>
                      )
                    )}
                  </td>
                  <td className="px-6 py-4 text-right">
                    <ChapterAction
                      chapter={chapter}
                      href={studioHref}
                      splitting={splittingIds.includes(chapter.id)}
                      onSplit={split}
                      onSynthesize={synthesize}
                      onCast={onGoToCast}
                      onProgress={() =>
                        show("本章正在推理集群中生成，可先试听已完成段落", "info")
                      }
                    />
                  </td>
                </tr>
              );
            })}

            {visible.length === 0 && (
              <tr>
                <td colSpan={6} className="px-6 py-14 text-center">
                  <p className="text-[13px] text-white/40">没有符合条件的章节</p>
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>

      {!expanded && collapsedCount > 0 && (
        <button
          type="button"
          onClick={() => setExpanded(true)}
          className="mt-3 flex w-full items-center justify-center gap-1.5 rounded-xl border border-dashed border-white/10 bg-ink-850 py-2.5 text-[12.5px] text-white/45 transition-colors hover:border-white/20 hover:text-white"
        >
          <ChevronDown className="h-3.5 w-3.5" />
          展开全部 {chapters.length} 章（其余 {collapsedCount} 章待拆解）
        </button>
      )}
    </div>
  );
}

function ChapterAction({
  chapter,
  href,
  splitting,
  onSplit,
  onSynthesize,
  onCast,
  onProgress,
}: {
  chapter: HubChapter;
  href: string;
  splitting: boolean;
  onSplit: (chapter: HubChapter) => void;
  onSynthesize: (chapter: HubChapter) => void;
  onCast: () => void;
  onProgress: () => void;
}) {
  const { action } = chapter;

  if (action.kind === "studio") {
    return (
      <Link
        href={href}
        className="inline-flex items-center gap-1.5 rounded-lg bg-accent px-3.5 py-1.5 text-[12.5px] font-medium text-white transition-colors hover:bg-accent-600"
      >
        <SlidersHorizontal className="h-3.5 w-3.5" />
        <span>进入制作工作台</span>
      </Link>
    );
  }

  if (action.kind === "progress") {
    return (
      <button
        type="button"
        onClick={onProgress}
        className="inline-flex items-center gap-1.5 rounded-lg border border-hairline bg-ink-850 px-3.5 py-1.5 text-[12.5px] text-white/45 transition-colors hover:bg-ink-750 hover:text-white/80"
      >
        <Hourglass className="h-3.5 w-3.5" />
        查看进度
      </button>
    );
  }

  if (action.kind === "split") {
    return (
      <button
        type="button"
        disabled={splitting}
        onClick={() => onSplit(chapter)}
        className="inline-flex items-center gap-1.5 rounded-lg border border-hairline bg-ink-850 px-3.5 py-1.5 text-[12.5px] text-white/50 transition-colors hover:bg-ink-750 hover:text-white/80 disabled:cursor-wait"
      >
        {splitting ? (
          <Loader className="h-3.5 w-3.5 animate-spin" />
        ) : (
          <WandSparkles className="h-3.5 w-3.5" />
        )}
        {splitting ? "拆解中" : "一键拆解"}
      </button>
    );
  }

  if (action.kind === "cast") {
    return (
      <button
        type="button"
        onClick={onCast}
        className="rounded-lg border border-amber-400/25 bg-amber-400/10 px-3.5 py-1.5 text-[12.5px] font-medium text-amber-200 transition-colors hover:bg-amber-400/20"
      >
        去选角
      </button>
    );
  }

  return (
    <div className="inline-flex items-center gap-1.5">
      <button
        type="button"
        onClick={() => onSynthesize(chapter)}
        className={
          action.primary
            ? "rounded-lg bg-accent px-3.5 py-1.5 text-[12.5px] font-medium text-white transition-colors hover:bg-accent-600"
            : "rounded-lg border border-hairline bg-ink-850 px-3.5 py-1.5 text-[12.5px] text-white/50 transition-colors hover:bg-accent hover:text-white"
        }
      >
        合成本章
      </button>
      {chapter.openable && (
        <Link
          href={href}
          title="进入制作工作台"
          className="rounded-lg border border-hairline bg-ink-850 p-1.5 text-white/45 transition-colors hover:text-white"
        >
          <SlidersHorizontal className="h-3.5 w-3.5" />
        </Link>
      )}
    </div>
  );
}
