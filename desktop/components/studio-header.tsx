"use client";

import {
  BadgeCheck,
  ChevronLeft,
  Coins,
  Download,
  Layers,
  Loader,
  MoreHorizontal,
  Play,
  RefreshCw,
  Search,
  Settings,
  Share,
} from "lucide-react";
import Link from "next/link";
import { useEffect, useRef, useState } from "react";
import { useToast } from "@/components/toast";
import { formatWords, type HubChapter, type StudioProject } from "@/lib/projects";

type SynthState = "idle" | "busy" | "done";

const iconButton =
  "flex h-8 w-8 shrink-0 items-center justify-center rounded-full border border-hairline bg-ink-800 text-white/55 transition-colors hover:bg-ink-700 hover:text-white";

export function StudioHeader({
  project,
  chapter,
  keyword,
  onKeywordChange,
  paragraphCount,
  cueCount,
}: {
  project: StudioProject;
  chapter: HubChapter;
  keyword: string;
  onKeywordChange: (value: string) => void;
  paragraphCount: number;
  cueCount: number;
}) {
  const { show } = useToast();
  const input = useRef<HTMLInputElement>(null);
  const timers = useRef<number[]>([]);
  const [menuOpen, setMenuOpen] = useState(false);
  const [synth, setSynth] = useState<SynthState>("idle");

  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        input.current?.focus();
        input.current?.select();
      }
      if (e.key === "Escape") setMenuOpen(false);
    }
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, []);

  useEffect(() => {
    if (!menuOpen) return;
    function onClick() {
      setMenuOpen(false);
    }
    document.addEventListener("click", onClick);
    return () => document.removeEventListener("click", onClick);
  }, [menuOpen]);

  // 卸载时收起未触发的提示定时器，避免在已销毁的组件上 setState
  useEffect(() => {
    const pending = timers.current;
    return () => pending.forEach(clearTimeout);
  }, []);

  function reparse() {
    show(`正在用 Qwen2.5 重新提取对白与情绪…`, "info");
    timers.current.push(
      window.setTimeout(
        () => show(`解析完成：${paragraphCount} 段对白 · ${cueCount} 处场景音效`, "success"),
        1400,
      ),
    );
  }

  function batchSynthesize() {
    if (synth !== "idle") return;
    setSynth("busy");
    timers.current.push(
      window.setTimeout(() => {
        setSynth("done");
        show(`当前章 ${paragraphCount} 个段落已通过 CosyVoice 2 推理完毕`, "success");
      }, 1600),
    );
    timers.current.push(window.setTimeout(() => setSynth("idle"), 5200));
  }

  async function shareChapter() {
    try {
      await navigator.clipboard.writeText(window.location.href);
    } catch {
      // 剪贴板不可用时仍按原型给出复制反馈
    }
    show("章节链接已复制到剪贴板", "success");
  }

  return (
    <header className="sticky top-0 z-30 flex h-16 shrink-0 items-center gap-3 border-b border-hairline bg-ink-900/85 px-4 backdrop-blur-md sm:gap-4 sm:px-6">
      {/* 面包屑：返回看板 + 剧目 + 当前章。原来的「切换剧目」是个假下拉，已去掉 */}
      <div className="flex min-w-0 flex-1 items-center gap-2.5">
        <Link
          href={`/project/${project.id}`}
          title="返回剧目看板"
          className={iconButton}
        >
          <ChevronLeft className="h-4 w-4" />
        </Link>

        <div className="flex min-w-0 items-center gap-2">
          <span
            className="flex h-6 w-6 shrink-0 items-center justify-center rounded-full bg-accent/15 text-[11px] font-bold text-accent-400"
            title={project.title}
          >
            {project.title.slice(0, 1)}
          </span>
        </div>

        <h1
          className="min-w-0 truncate text-[13.5px] font-semibold text-white"
          title={chapter.title}
        >
          {chapter.title}
        </h1>

        <span className="hidden shrink-0 rounded-full border border-hairline bg-ink-800 px-2 py-0.5 font-mono text-[10.5px] text-white/40 lg:inline">
          {chapter.words ? `${formatWords(chapter.words)} 字` : "待统计"}
        </span>
      </div>

      <div className="relative hidden w-[200px] shrink-0 md:block xl:w-[280px]">
        <Search className="pointer-events-none absolute left-3 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-white/25" />
        <input
          ref={input}
          type="text"
          value={keyword}
          onChange={(e) => onKeywordChange(e.target.value)}
          onKeyDown={(e) => {
            if (e.key !== "Escape") return;
            onKeywordChange("");
            e.currentTarget.blur();
          }}
          placeholder="搜索段落、角色…"
          className="h-8 w-full rounded-full border border-hairline bg-ink-800 pl-8 pr-11 text-[12.5px] text-white placeholder:text-white/30 focus:border-accent/50 focus:bg-ink-750 focus:outline-none"
        />
        <kbd className="pointer-events-none absolute right-2 top-1/2 -translate-y-1/2 rounded border border-white/10 bg-white/6 px-1.5 py-0.5 font-mono text-[10px] text-white/40">
          ⌘K
        </kbd>
      </div>

      <div className="flex shrink-0 items-center gap-1">
        <button
          type="button"
          title="复制章节链接"
          onClick={shareChapter}
          className={iconButton}
        >
          <Share className="h-4 w-4" />
        </button>

        <div className="relative">
          <button
            type="button"
            title="更多操作"
            onClick={(e) => {
              e.stopPropagation();
              setMenuOpen((open) => !open);
            }}
            className={iconButton}
          >
            <MoreHorizontal className="h-4 w-4" />
          </button>

          {menuOpen && (
            <div className="absolute right-0 top-10 z-40 w-44 overflow-hidden rounded-xl border border-hairline bg-ink-750 py-1 shadow-2xl shadow-black/60">
              <button
                type="button"
                onClick={reparse}
                className="flex w-full items-center gap-2.5 px-3 py-2 text-left text-[12.5px] text-white transition-colors hover:bg-white/5"
              >
                <RefreshCw className="h-3.5 w-3.5 opacity-60" />
                重新解析剧本
              </button>
              <button
                type="button"
                onClick={() => show("已把本章加入批量产量队列", "success")}
                className="flex w-full items-center gap-2.5 px-3 py-2 text-left text-[12.5px] text-white transition-colors hover:bg-white/5"
              >
                <Layers className="h-3.5 w-3.5 opacity-60" />
                加入批量队列
              </button>
              <button
                type="button"
                onClick={() => show("已打开章节设置", "info")}
                className="flex w-full items-center gap-2.5 px-3 py-2 text-left text-[12.5px] text-white transition-colors hover:bg-white/5"
              >
                <Settings className="h-3.5 w-3.5 opacity-60" />
                章节设置
              </button>
            </div>
          )}
        </div>
      </div>

      <div className="flex shrink-0 items-center gap-2">
        <Link
          href={`/project/${project.id}/export`}
          className="flex items-center gap-2 rounded-full border border-hairline bg-ink-800 px-3 py-2 text-[12.5px] font-medium text-white/75 transition-colors hover:border-emerald-400/30 hover:bg-emerald-400/10 hover:text-emerald-300"
        >
          <Download className="h-4 w-4 text-emerald-400/80" />
          <span className="hidden md:inline">导出本章</span>
        </Link>

        <button
          type="button"
          onClick={batchSynthesize}
          className="flex items-center gap-2 rounded-full bg-accent px-3.5 py-2 text-[12.5px] font-medium text-white transition-colors hover:bg-accent-600"
        >
          {synth === "busy" ? (
            <>
              <Loader className="h-4 w-4 animate-spin" />
              <span>GPU 推理中…</span>
            </>
          ) : synth === "done" ? (
            <>
              <BadgeCheck className="h-4 w-4" />
              <span>本章合成完成</span>
            </>
          ) : (
            <>
              <Play className="h-4 w-4" />
              <span className="hidden sm:inline">批量合成当前章</span>
            </>
          )}
        </button>

        <span
          title="剩余额度"
          className="hidden items-center gap-1.5 rounded-full border border-hairline bg-ink-800 px-3 py-2 text-[12.5px] text-white/80 transition-colors hover:border-white/20 xl:flex"
        >
          <Coins className="h-3.5 w-3.5 text-white/40" />
          <span className="font-mono">6,240</span>
          <span className="rounded bg-white/10 px-1.5 py-0.5 text-[10px] font-semibold text-white/60">
            PRO
          </span>
        </span>

        <button
          type="button"
          className="hidden h-8 w-8 shrink-0 rounded-full bg-ink-700 text-[12px] font-bold text-white/70 ring-1 ring-white/10 transition-colors hover:ring-white/25 sm:block"
        >
          L
        </button>
      </div>
    </header>
  );
}
