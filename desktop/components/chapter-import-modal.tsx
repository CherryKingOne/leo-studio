"use client";

import { FilePlus2, FileText, Upload, X } from "lucide-react";
import { useState } from "react";
import { fieldLabel, Modal, primaryButton, secondaryButton } from "@/components/modal";
import { appendChaptersAction } from "@/lib/actions";
import { formatWords } from "@/lib/projects";
import { inkScrollbar } from "@/lib/scrollbar";
import type { ChapterAppendResult } from "@/lib/api";

const maxSize = 50 * 1024 * 1024;

function formatSize(bytes: number) {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / 1024 / 1024).toFixed(2)} MB`;
}

/**
 * 追更上传章节文件：分章与同名比对都交给后端，
 * 选完文件先跑一次 dry_run 预检渲染摘要，确认后才真正上传。
 */
export function ChapterImportModal({
  projectId,
  chapterCount,
  onClose,
  onImported,
}: {
  projectId: string;
  chapterCount: number;
  onClose: () => void;
  onImported: (result: ChapterAppendResult) => void;
}) {
  const [file, setFile] = useState<File | null>(null);
  const [overwriteExisting, setOverwriteExisting] = useState(false);
  const [preview, setPreview] = useState<ChapterAppendResult | null>(null);
  const [pending, setPending] = useState<"preview" | "commit" | null>(null);
  const [failure, setFailure] = useState<string | null>(null);
  const [dragging, setDragging] = useState(false);

  async function send(target: File, dryRun: boolean) {
    setPending(dryRun ? "preview" : "commit");
    setFailure(null);

    const form = new FormData();
    form.set("projectId", projectId);
    form.set("file", target, target.name);
    form.set("overwriteExisting", String(overwriteExisting));
    form.set("dryRun", String(dryRun));

    const result = await appendChaptersAction(form);
    setPending(null);
    if (!result.ok) {
      setFailure(result.message);
      setPreview(null);
      return;
    }
    setPreview(result.preview);
    if (!dryRun) {
      onImported(result.preview);
      onClose();
    }
  }

  function pick(list: FileList | null) {
    const chosen = list?.[0];
    if (!chosen) return;
    if (!/\.txt$/i.test(chosen.name)) {
      setFailure("后端目前只接受 .txt 正文，请换文件");
      return;
    }
    if (chosen.size > maxSize) {
      setFailure(`${chosen.name} 超过 50 MB，已跳过`);
      return;
    }
    setFile(chosen);
    void send(chosen, true);
  }

  const busy = pending !== null;
  // 覆盖模式下后端只报 skipped/冲突、不报新增，此时可执行的动作是「覆盖」
  const overwritable = overwriteExisting && (preview?.skippedChapters ?? 0) > 0;
  const submittable = !!preview && (preview.addedChapters > 0 || overwritable);
  const submitLabel = !preview || preview.addedChapters === 0
    ? overwritable
      ? `覆盖 ${preview?.skippedChapters} 章`
      : "导入章节"
    : `导入 ${preview.addedChapters} 个新章节${overwritable ? ` · 覆盖 ${preview.skippedChapters} 章` : ""}`;

  return (
    <Modal
      title="追更上传章节文件"
      description="连载中的书可随更新分批导入，新章节自动接在现有最后一章之后 · 支持 .txt"
      size="lg"
      onClose={onClose}
      footer={
        <div className="flex items-center justify-between gap-3 border-t border-hairline px-6 py-4">
          <span className="font-mono text-[11px] text-white/35">
            当前 {chapterCount} 章
            {preview ? ` · 导入后 ${preview.totalChapters} 章` : ""}
          </span>
          <div className="flex items-center gap-2">
            <button type="button" onClick={onClose} className={secondaryButton}>
              取消
            </button>
            <button
              type="button"
              disabled={!file || !submittable || busy}
              onClick={() => file && void send(file, false)}
              className={primaryButton}
            >
              <Upload className="h-3.5 w-3.5" />
              <span>{pending === "commit" ? "上传中" : submitLabel}</span>
            </button>
          </div>
        </div>
      }
    >
      <div className="space-y-5 px-6 py-5">
        <div>
          <span className={fieldLabel}>章节文件</span>
          <div
            role="button"
            tabIndex={0}
            onClick={() => document.getElementById("chapter-file")?.click()}
            onKeyDown={(e) => {
              if (e.key === "Enter" || e.key === " ") document.getElementById("chapter-file")?.click();
            }}
            onDragOver={(e) => {
              e.preventDefault();
              setDragging(true);
            }}
            onDragLeave={() => setDragging(false)}
            onDrop={(e) => {
              e.preventDefault();
              setDragging(false);
              pick(e.dataTransfer.files);
            }}
            className={`cursor-pointer rounded-xl border border-dashed px-6 py-8 text-center transition-colors ${
              dragging
                ? "border-accent/60 bg-accent/6"
                : "border-white/12 bg-ink-900/60 hover:border-accent/50"
            }`}
          >
            <FilePlus2 className="mx-auto mb-2.5 h-6 w-6 text-white/30" />
            <div className="text-[13px] text-white/75">点击选择，或把章节文件拖拽到这里</div>
            <div className="mt-1 text-[11px] text-white/35">
              单文件最大 50 MB · 章节名以「第X章」开头最易自动识别
            </div>
          </div>
          <input
            id="chapter-file"
            type="file"
            accept=".txt"
            className="hidden"
            onChange={(e) => {
              pick(e.target.files);
              e.target.value = "";
            }}
          />

          {file && (
            <div className="mt-3 flex items-center justify-between gap-3 rounded-xl border border-hairline bg-ink-850/70 px-3.5 py-2.5">
              <div className="flex min-w-0 items-center gap-2.5">
                <FileText className="h-4 w-4 shrink-0 text-white/35" />
                <div className="min-w-0">
                  <div className="truncate text-[12.5px] text-white/80">{file.name}</div>
                  <div className="mt-0.5 font-mono text-[10.5px] text-white/30">
                    {formatSize(file.size)}
                    {pending === "preview" ? " · 预检中…" : ""}
                  </div>
                </div>
              </div>
              <button
                type="button"
                title="移除该文件"
                onClick={() => {
                  setFile(null);
                  setPreview(null);
                  setFailure(null);
                }}
                className="shrink-0 rounded-md p-1.5 text-white/35 hover:bg-white/5 hover:text-white"
              >
                <X className="h-3.5 w-3.5" />
              </button>
            </div>
          )}
        </div>

        {preview && (
          <div className="rounded-xl border border-hairline bg-ink-850/70 px-3.5 py-3">
            <div className="mb-2 flex items-center justify-between gap-3">
              <span className={`${fieldLabel} mb-0`}>预检结果</span>
              <span className="font-mono text-[10.5px] text-white/35">
                新增 {preview.addedChapters} 章 · 跳过同名 {preview.skippedChapters} 章
              </span>
            </div>
            <p className="text-[12px] leading-relaxed text-white/55">
              {preview.addedChapters > 0
                ? `将追加 ${preview.addedChapters} 个新章节，接在第 ${preview.totalChapters - preview.addedChapters} 章之后；导入后章节状态为待拆解。`
                : overwritable
                  ? `没有新章节，将覆盖 ${preview.skippedChapters} 章同名正文并重置为待拆解。`
                  : "这份文件里没有新章节可追加，换个文件或开启覆盖同名章节。"}
            </p>

            {preview.conflicts.length > 0 && (
              <div className="mt-3">
                <div className="mb-1.5 text-[11.5px] text-amber-300/80">
                  检出 {preview.conflicts.length} 章同名但字数不一致
                </div>
                <div
                  className={`${inkScrollbar} max-h-36 overflow-y-auto rounded-lg border border-hairline bg-ink-900/60`}
                >
                  {preview.conflicts.map((item) => (
                    <div
                      key={item.chapterTitle}
                      className="flex items-center justify-between gap-3 border-b border-hairline px-3 py-2 last:border-b-0"
                    >
                      <span className="truncate text-[12px] text-white/70">
                        {item.chapterTitle}
                      </span>
                      <span className="shrink-0 font-mono text-[10.5px] text-amber-300/80">
                        {formatWords(item.existingWordCount)} → {formatWords(item.incomingWordCount)} 字
                        {item.wordDiff > 0 ? " +" : " "}
                        {item.wordDiff}
                      </span>
                    </div>
                  ))}
                </div>
              </div>
            )}
          </div>
        )}

        <label className="flex cursor-pointer items-start gap-2.5 rounded-xl border border-hairline bg-ink-850/70 px-3.5 py-3">
          <input
            type="checkbox"
            checked={overwriteExisting}
            disabled={busy}
            onChange={(e) => {
              const next = e.target.checked;
              setOverwriteExisting(next);
              if (file) {
                setOverwriteExisting(next);
                void send(file, true);
              }
            }}
            className="mt-0.5 size-3.5 accent-accent"
          />
          <span className="text-[12px] leading-relaxed text-white/60">
            覆盖已有同名章节
            <br />
            <span className="text-white/35">
              关闭时只追加新章节，已有章节的正文与校对结果不受影响
            </span>
          </span>
        </label>

        {failure && <p className="text-[12.5px] text-red-400">{failure}</p>}
      </div>
    </Modal>
  );
}
