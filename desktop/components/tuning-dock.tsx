"use client";

import {
  Dices,
  Mic,
  MoreVertical,
  PanelRightClose,
  RotateCw,
  SlidersHorizontal,
  Volume2,
  WandSparkles,
} from "lucide-react";
import Link from "next/link";
import { useState } from "react";
import { useToast } from "@/components/toast";
import { inkScrollbar } from "@/lib/scrollbar";
import type { StudioParagraph } from "@/lib/projects";

const field =
  "w-full rounded-lg border border-hairline bg-ink-900 px-2.5 py-2 text-[12.5px] text-white [color-scheme:dark] focus:border-accent/60 focus:outline-none";

const dockButton =
  "flex h-8 w-8 items-center justify-center rounded-full text-white/50 transition-colors hover:bg-white/7 hover:text-white";

export function TuningDock({
  open,
  paragraph,
  seq,
  speakerOptions,
  modelOptions,
  reverbOptions,
  onOpenChange,
  onAudition,
}: {
  open: boolean;
  paragraph: StudioParagraph;
  seq: number;
  speakerOptions: string[];
  modelOptions: string[];
  reverbOptions: string[];
  onOpenChange: (open: boolean) => void;
  onAudition: (hz: number, seconds: number) => void;
}) {
  const { show } = useToast();

  return (
    <div className="pointer-events-none absolute inset-y-0 right-0 z-30 hidden md:block">
      {open ? (
        // key 让面板在切换段落时按新段落的参数重建，等价于原型的重新绑定
        <TuningPanel
          key={paragraph.id}
          paragraph={paragraph}
          seq={seq}
          speakerOptions={speakerOptions}
          modelOptions={modelOptions}
          reverbOptions={reverbOptions}
          onOpenChange={onOpenChange}
        />
      ) : (
        <div className="pointer-events-auto absolute right-4 top-1/2 flex -translate-y-1/2 flex-col items-center gap-1 rounded-full border border-hairline bg-ink-800/95 p-1.5 shadow-2xl shadow-black/60 backdrop-blur-md">
          <button
            type="button"
            title="展开配音设置"
            onClick={() => onOpenChange(true)}
            className="flex h-9 w-9 items-center justify-center rounded-full bg-accent text-white transition-colors hover:bg-accent-600"
          >
            <SlidersHorizontal className="h-4 w-4" />
          </button>

          <div className="my-1 h-px w-6 bg-white/7" />

          <button
            type="button"
            title="试听当前段"
            onClick={() => onAudition(paragraph.hz, paragraph.seconds)}
            className={dockButton}
          >
            <Volume2 className="h-4 w-4" />
          </button>
          <button
            type="button"
            title="局部重绘本段"
            onClick={() => show("正在局部重新推理，仅覆盖当前段落", "info")}
            className={dockButton}
          >
            <WandSparkles className="h-4 w-4" />
          </button>
          <Link href="/voice-library" title="更换音色" className={dockButton}>
            <Mic className="h-4 w-4" />
          </Link>
          <button
            type="button"
            title="重新生成"
            onClick={() => show("已重新加入生成队列", "info")}
            className={dockButton}
          >
            <RotateCw className="h-4 w-4" />
          </button>

          <div className="my-1 h-px w-6 bg-white/7" />

          <button
            type="button"
            title="更多参数"
            onClick={() => onOpenChange(true)}
            className={dockButton}
          >
            <MoreVertical className="h-4 w-4" />
          </button>
        </div>
      )}
    </div>
  );
}

function TuningPanel({
  paragraph,
  seq,
  speakerOptions,
  modelOptions,
  reverbOptions,
  onOpenChange,
}: {
  paragraph: StudioParagraph;
  seq: number;
  speakerOptions: string[];
  modelOptions: string[];
  reverbOptions: string[];
  onOpenChange: (open: boolean) => void;
}) {
  const { show } = useToast();
  const tuning = paragraph.tuning;
  const [emotion, setEmotion] = useState(tuning.emotion);
  const [speed, setSpeed] = useState(tuning.speed);
  const [seed, setSeed] = useState(tuning.seed);

  function rollSeed() {
    const next = String(Math.floor(1000 + Math.random() * 8999));
    setSeed(next);
    show(`已换 Seed ${next}，可局部重绘本段`, "info");
  }

  return (
    <div className={`pointer-events-auto absolute inset-y-4 right-4 w-72 overflow-y-auto ${inkScrollbar}`}>
      <div className="flex min-h-full flex-col rounded-2xl border border-hairline bg-ink-800 p-5 shadow-2xl shadow-black/70">
        <div className="flex items-start justify-between gap-2 border-b border-hairline pb-3">
          <div className="min-w-0">
            <span className="block text-[12.5px] font-semibold text-white">配音设置</span>
            <span className="mt-1 block font-mono text-[10.5px] text-white/30">
              #{String(seq).padStart(3, "0")} {paragraph.speaker}
            </span>
          </div>
          <button
            type="button"
            title="收起为悬浮工具栏"
            onClick={() => onOpenChange(false)}
            className="shrink-0 rounded-lg border border-hairline bg-ink-850 p-1.5 text-white/50 transition-colors hover:border-white/20 hover:text-white"
          >
            <PanelRightClose className="h-4 w-4" />
          </button>
        </div>

        <div className="flex-1 space-y-5 pt-4">
          <div>
            <label className="mb-1.5 block text-[11.5px] font-medium text-white/60">说话人</label>
            <select
              defaultValue={tuning.speaker}
              onChange={(e) => show(`本段说话人已改绑为「${e.target.value}」，仅影响后续合成`, "success")}
              className={field}
            >
              {speakerOptions.map((option) => (
                <option key={option} value={option}>
                  {option}
                </option>
              ))}
            </select>
          </div>

          <div>
            <label className="mb-1.5 block text-[11.5px] font-medium text-white/60">音色模型</label>
            <select defaultValue={tuning.model} className={field}>
              {modelOptions.map((model) => (
                <option key={model} value={model}>
                  {model}
                </option>
              ))}
            </select>
            <Link
              href="/voice-library"
              className="mt-1.5 block text-[11px] text-white/35 transition-colors hover:text-accent-400"
            >
              前往全局声线资产库
            </Link>
          </div>

          <div>
            <div className="mb-2 flex items-center justify-between">
              <label className="text-[11.5px] font-medium text-white/60">情绪强度</label>
              <span className="font-mono text-[11px] text-amber-300">{emotion}%</span>
            </div>
            <input
              type="range"
              min={0}
              max={100}
              value={emotion}
              onChange={(e) => setEmotion(Number(e.target.value))}
              className="w-full accent-accent"
            />
            <div className="mt-1 flex justify-between text-[10.5px] text-white/30">
              <span>平缓</span>
              <span>激烈</span>
            </div>
          </div>

          <div>
            <div className="mb-2 flex items-center justify-between">
              <label className="text-[11.5px] font-medium text-white/60">语速</label>
              <span className="font-mono text-[11px] text-white/60">{speed.toFixed(2)}x</span>
            </div>
            <input
              type="range"
              min={0.5}
              max={2}
              step={0.05}
              value={speed}
              onChange={(e) => setSpeed(Number(e.target.value))}
              className="w-full accent-accent"
            />
          </div>

          <div>
            <label className="mb-1.5 block text-[11.5px] font-medium text-white/60">空间混响</label>
            <select defaultValue={tuning.reverb} className={field}>
              {reverbOptions.map((reverb) => (
                <option key={reverb} value={reverb}>
                  {reverb}
                </option>
              ))}
            </select>
          </div>

          <div>
            <label className="mb-1.5 block text-[11.5px] font-medium text-white/60">推理 Seed</label>
            <div className="flex items-center gap-2">
              <input
                type="text"
                value={seed}
                onChange={(e) => setSeed(e.target.value)}
                className={`${field} font-mono`}
              />
              <button
                type="button"
                title="换随机 Seed"
                onClick={rollSeed}
                className="shrink-0 rounded-lg border border-hairline bg-ink-850 p-2 text-white/50 transition-colors hover:border-white/20 hover:bg-ink-750 hover:text-white"
              >
                <Dices className="h-4 w-4" />
              </button>
            </div>
          </div>
        </div>

        <div className="mt-5 shrink-0 space-y-2 border-t border-hairline pt-4">
          <button
            type="button"
            onClick={() => show("正在局部重新推理，仅覆盖当前段落", "info")}
            className="flex w-full items-center justify-center gap-2 rounded-lg bg-accent py-2 text-[12.5px] font-medium text-white transition-colors hover:bg-accent-600"
          >
            <WandSparkles className="h-4 w-4" />
            局部重绘本段
          </button>
          <button
            type="button"
            onClick={() => show("已重新加入生成队列", "info")}
            className="flex w-full items-center justify-center gap-2 rounded-lg border border-hairline bg-ink-850 py-2 text-[12.5px] text-white/70 transition-colors hover:border-white/20 hover:bg-ink-750 hover:text-white"
          >
            <RotateCw className="h-4 w-4" />
            重新生成
          </button>
        </div>
      </div>
    </div>
  );
}
