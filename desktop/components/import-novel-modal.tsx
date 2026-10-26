"use client";

import { ArrowRight, Upload, X } from "lucide-react";
import {
  createContext,
  useContext,
  useEffect,
  useRef,
  useState,
  type ReactNode,
} from "react";

type ImportNovelModalValue = { open: () => void };

const ImportNovelModalContext = createContext<ImportNovelModalValue | null>(null);

export function useImportNovelModal() {
  const value = useContext(ImportNovelModalContext);
  if (!value) {
    throw new Error("useImportNovelModal 必须在 ImportNovelModalProvider 内使用");
  }
  return value;
}

const fieldLabel = "mb-1.5 block text-[12.5px] font-medium text-white/75";
const fieldControl =
  "w-full rounded-lg border border-hairline bg-ink-900 px-3 py-2.5 text-[13px] text-white placeholder:text-white/25 focus:border-accent/60 focus:outline-none";

export function ImportNovelModalProvider({
  children,
  folders,
  models,
}: {
  children: ReactNode;
  folders: string[];
  models: string[];
}) {
  const [isOpen, setIsOpen] = useState(false);

  useEffect(() => {
    if (!isOpen) return;
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") setIsOpen(false);
    };
    document.addEventListener("keydown", onKeyDown);
    return () => document.removeEventListener("keydown", onKeyDown);
  }, [isOpen]);

  return (
    <ImportNovelModalContext.Provider value={{ open: () => setIsOpen(true) }}>
      {children}
      {isOpen && (
        <ImportNovelModal
          folders={folders}
          models={models}
          onClose={() => setIsOpen(false)}
        />
      )}
    </ImportNovelModalContext.Provider>
  );
}

export function ImportNovelModal({
  folders,
  models,
  onClose,
}: {
  folders: string[];
  models: string[];
  onClose: () => void;
}) {
  const [manuscript, setManuscript] = useState<string | null>(null);
  const titleRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    titleRef.current?.focus();
  }, []);

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-label="导入小说，创建有声剧目"
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/75 p-4 backdrop-blur-sm"
    >
      <div className="w-full max-w-[520px] rounded-2xl border border-hairline bg-ink-800 shadow-2xl shadow-black/70">
        <div className="flex items-center justify-between border-b border-hairline px-6 py-4">
          <div>
            <h2 className="text-[15px] font-semibold text-white">
              导入小说，创建有声剧目
            </h2>
            <p className="mt-0.5 text-[11.5px] text-white/40">
              自动识别章节标记，拆分对白与旁白并建立生产流水线
            </p>
          </div>
          <button
            type="button"
            onClick={onClose}
            className="rounded-md p-1.5 text-white/40 hover:bg-white/5 hover:text-white"
            title="关闭"
          >
            <X className="h-4 w-4" />
          </button>
        </div>

        <form
          onSubmit={(e) => {
            e.preventDefault();
            onClose();
          }}
          className="space-y-5 px-6 py-5"
        >
          <div>
            <label className={fieldLabel} htmlFor="project-title">
              剧目名称
            </label>
            <input
              id="project-title"
              ref={titleRef}
              type="text"
              placeholder="例如：诡秘之主：第一部 愚者"
              className={fieldControl}
            />
          </div>

          <div>
            <span className={fieldLabel}>电子书文件（.txt / .epub）</span>
            <label className="block cursor-pointer rounded-xl border border-dashed border-white/12 bg-ink-900/60 px-6 py-8 text-center transition-colors hover:border-accent/50">
              <Upload className="mx-auto mb-2.5 h-6 w-6 text-white/30" />
              <div className="text-[13px] text-white/75">
                {manuscript ?? "点击或拖拽文件到此处"}
              </div>
              <div className="mt-1 text-[11px] text-white/35">
                单本最大 20 MB · 解析后可在剧目看板校对章节
              </div>
              <input
                type="file"
                accept=".txt,.epub"
                className="sr-only"
                onChange={(e) =>
                  setManuscript(e.target.files?.[0]?.name ?? null)
                }
              />
            </label>
          </div>

          <div className="grid grid-cols-2 gap-4">
            <div>
              <label className={fieldLabel} htmlFor="tts-model">
                默认 TTS 模型
              </label>
              <select id="tts-model" className={fieldControl}>
                {models.map((model) => (
                  <option key={model}>{model}</option>
                ))}
              </select>
            </div>
            <div>
              <label className={fieldLabel} htmlFor="target-folder">
                归属文件夹
              </label>
              <select id="target-folder" className={fieldControl}>
                <option>根目录</option>
                {folders.map((folder) => (
                  <option key={folder}>{folder}</option>
                ))}
              </select>
            </div>
          </div>

          <div className="flex items-center justify-end gap-2 border-t border-hairline px-6 py-4" >
            <button
              type="button"
              onClick={onClose}
              className="rounded-lg border border-hairline px-4 py-2 text-[13px] text-white/65 hover:bg-white/5 hover:text-white"
            >
              取消
            </button>
            <button
              type="submit"
              className="flex items-center gap-1.5 rounded-lg bg-accent px-4 py-2 text-[13px] font-medium text-white hover:bg-accent-600"
            >
              开始解析并进入剧目
              <ArrowRight className="h-4 w-4" />
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
