"use client";

import { FilePlus2, FileText, Upload, X } from "lucide-react";
import { useMemo, useRef, useState } from "react";
import {
  fieldLabel,
  Modal,
  primaryButton,
  secondaryButton,
} from "@/components/modal";
import { useToast } from "@/components/toast";
import {
  formatWords,
  nextChapterNo,
  type HubChapter,
  type ImportedChapterDraft,
} from "@/lib/projects";
import { inkScrollbar } from "@/lib/scrollbar";

type ParsedPart = { title: string; words: number; excerpt: string };
type ParsedFile = { name: string; size: number; parts: ParsedPart[] };

const maxSize = 20 * 1024 * 1024;
const textFile = /\.(txt|md)$/i;
const acceptedFile = /\.(txt|md|epub|docx?)$/i;
// 「第二章节奏很快」这类正文首句会被标题正则误判，故标题行必须短且不带句读
const chapterHeading = /^第\s*[0-9零一二三四五六七八九十百千万两〇]{1,12}\s*[章节回]/;

const stem = (name: string) => name.replace(/\.[^.]+$/, "");

const wholeFile = (name: string): ParsedPart => ({
  title: stem(name),
  words: 0,
  excerpt: "整文件作为一章，拆解时自动分章",
});

function headingOf(line: string) {
  const text = line.trim();
  const punctuated = /[。！？!?；;，,]/.test(text);
  return chapterHeading.test(text) && text.length <= 30 && !punctuated ? text : null;
}

function splitChapters(name: string, raw: string): ParsedPart[] {
  const found: { title: string; body: string[] }[] = [];
  let current: { title: string; body: string[] } | null = null;

  for (const line of raw.replace(/\r\n?/g, "\n").split("\n")) {
    const title = headingOf(line);
    if (title) {
      current = { title, body: [] };
      found.push(current);
    } else if (current) {
      current.body.push(line);
    }
  }

  const parts = found
    .map(({ title, body }) => {
      const text = body.join("\n").trim();
      const excerpt =
        text
          .split("\n")
          .map((line) => line.trim())
          .find((line) => line.length > 6) ?? "";
      return { title, words: text.replace(/\s+/g, "").length, excerpt: excerpt.slice(0, 40) };
    })
    .filter((part) => part.words > 20);

  return parts.length ? parts : [wholeFile(name)];
}

function formatSize(bytes: number) {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
}

export function ChapterImportModal({
  chapters,
  onClose,
  onImport,
}: {
  chapters: HubChapter[];
  onClose: () => void;
  onImport: (drafts: ImportedChapterDraft[]) => void;
}) {
  const { show } = useToast();
  const fileInput = useRef<HTMLInputElement>(null);
  const [files, setFiles] = useState<ParsedFile[]>([]);
  const [dragging, setDragging] = useState(false);

  const nextNo = nextChapterNo(chapters);
  const existing = useMemo(
    () => new Set(chapters.map((chapter) => chapter.title)),
    [chapters],
  );

  const plan = useMemo(() => {
    const seen = new Set<string>();
    const items: ImportedChapterDraft[] = [];
    let skipped = 0;

    for (const file of files) {
      for (const part of file.parts) {
        if (seen.has(part.title)) continue;
        seen.add(part.title);
        if (existing.has(part.title)) skipped += 1;
        else items.push(part);
      }
    }
    return { items, skipped };
  }, [files, existing]);

  const words = plan.items.reduce((sum, item) => sum + item.words, 0);
  const submitLabel = plan.items.length
    ? `导入 ${plan.items.length} 个新章节`
    : "导入章节";

  async function addFiles(list: FileList | null) {
    if (!list?.length) return;
    const all = Array.from(list);
    const accepted = all.filter((file) => acceptedFile.test(file.name));
    if (accepted.length < all.length) {
      show(`${all.length - accepted.length} 个文件类型不支持，已忽略`, "info");
    }

    const parsed: ParsedFile[] = [];
    for (const file of accepted) {
      if (file.size > maxSize) {
        show(`${file.name} 超过 20 MB，已跳过`, "info");
        continue;
      }
      let parts: ParsedPart[] = [];
      if (textFile.test(file.name)) {
        try {
          parts = splitChapters(file.name, await file.text());
        } catch {
          parts = [];
        }
      }
      parsed.push({ name: file.name, size: file.size, parts });
    }
    if (parsed.length) setFiles((prev) => [...prev, ...parsed]);
  }

  return (
    <Modal
      title="追更上传章节文件"
      description="连载中的书可随更新分批导入，新章节自动接在现有最后一章之后 · 支持 .txt / .md / .epub / .docx"
      size="lg"
      onClose={onClose}
      footer={
        <div className="flex items-center justify-between gap-3 border-t border-hairline px-6 py-4">
          <span className="font-mono text-[11px] text-white/35">
            当前 {nextNo - 1} 章 · 新章节从第 {String(nextNo).padStart(3, "0")} 章续编
          </span>
          <div className="flex items-center gap-2">
            <button type="button" onClick={onClose} className={secondaryButton}>
              取消
            </button>
            <button
              type="button"
              disabled={!plan.items.length}
              onClick={() => {
                onImport(plan.items);
                onClose();
              }}
              className={primaryButton}
            >
              <Upload className="h-3.5 w-3.5" />
              <span>{submitLabel}</span>
            </button>
          </div>
        </div>
      }
    >
      <div className="space-y-5 px-6 py-5">
        <div>
          <span className={fieldLabel}>章节文件（可多选）</span>
          <div
            role="button"
            tabIndex={0}
            onClick={() => fileInput.current?.click()}
            onKeyDown={(e) => {
              if (e.key === "Enter" || e.key === " ") fileInput.current?.click();
            }}
            onDragOver={(e) => {
              e.preventDefault();
              setDragging(true);
            }}
            onDragLeave={() => setDragging(false)}
            onDrop={(e) => {
              e.preventDefault();
              setDragging(false);
              void addFiles(e.dataTransfer.files);
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
              单文件最大 20 MB · 建议 UTF-8 纯文本，章节名以「第X章」开头最易自动识别
            </div>
          </div>
          <input
            ref={fileInput}
            type="file"
            multiple
            accept=".txt,.md,.epub,.docx,.doc"
            className="hidden"
            onChange={(e) => {
              void addFiles(e.target.files);
              e.target.value = "";
            }}
          />

          {files.length > 0 && (
            <div className="mt-3 space-y-2">
              {files.map((file, index) => (
                <div
                  key={`${file.name}-${index}`}
                  className="flex items-center justify-between gap-3 rounded-xl border border-hairline bg-ink-850/70 px-3.5 py-2.5"
                >
                  <div className="flex min-w-0 items-center gap-2.5">
                    <FileText className="h-4 w-4 shrink-0 text-white/35" />
                    <div className="min-w-0">
                      <div className="truncate text-[12.5px] text-white/80">{file.name}</div>
                      <div className="mt-0.5 font-mono text-[10.5px] text-white/30">
                        {formatSize(file.size)} · 识别到 {file.parts.length} 章
                      </div>
                    </div>
                  </div>
                  <button
                    type="button"
                    title="移除该文件"
                    onClick={() => setFiles((prev) => prev.filter((_, i) => i !== index))}
                    className="shrink-0 rounded-md p-1.5 text-white/35 hover:bg-white/5 hover:text-white"
                  >
                    <X className="h-3.5 w-3.5" />
                  </button>
                </div>
              ))}
            </div>
          )}
        </div>

        {(plan.items.length > 0 || plan.skipped > 0) && (
          <div>
            <div className="mb-2 flex items-center justify-between gap-3">
              <span className={`${fieldLabel} mb-0`}>将要导入的章节</span>
              <span className="font-mono text-[10.5px] text-white/35">
                {plan.items.length} 章
                {words ? ` · 约 ${formatWords(words)} 字` : ""}
                {plan.skipped ? ` · 跳过 ${plan.skipped} 章同名` : ""}
              </span>
            </div>
            <div
              className={`max-h-44 overflow-y-auto rounded-xl border border-hairline bg-ink-850/70 ${inkScrollbar}`}
            >
              {plan.items.map((item, index) => (
                <div
                  key={item.title}
                  className="flex items-center justify-between gap-3 border-b border-hairline px-3.5 py-2 last:border-b-0"
                >
                  <div className="flex min-w-0 items-center gap-2.5">
                    <span className="shrink-0 font-mono text-[10.5px] text-white/30">
                      {String(nextNo + index).padStart(3, "0")}
                    </span>
                    <span className="truncate text-[12px] text-white/75">{item.title}</span>
                  </div>
                  <span className="shrink-0 font-mono text-[10.5px] text-white/35">
                    {item.words ? `${formatWords(item.words)} 字` : "待统计"}
                  </span>
                </div>
              ))}
              {plan.skipped > 0 && (
                <div className="px-3.5 py-2 text-[11px] text-white/35">
                  另有 {plan.skipped} 章与看板已有章节同名，未重复导入。
                </div>
              )}
            </div>
          </div>
        )}
      </div>
    </Modal>
  );
}
