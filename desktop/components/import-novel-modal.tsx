"use client";

import { ArrowRight, Upload } from "lucide-react";
import { createContext, useContext, useState, type ReactNode } from "react";
import {
  fieldControl,
  fieldLabel,
  Modal,
  modalBody,
  modalFooter,
  primaryButton,
  secondaryButton,
} from "@/components/modal";

type ImportNovelModalValue = { open: () => void };

const ImportNovelModalContext = createContext<ImportNovelModalValue | null>(null);

export function useImportNovelModal() {
  const value = useContext(ImportNovelModalContext);
  if (!value) {
    throw new Error("useImportNovelModal 必须在 ImportNovelModalProvider 内使用");
  }
  return value;
}

export function ImportNovelModalProvider({ children }: { children: ReactNode }) {
  const [isOpen, setIsOpen] = useState(false);

  return (
    <ImportNovelModalContext.Provider value={{ open: () => setIsOpen(true) }}>
      {children}
      {isOpen && <ImportNovelModal onClose={() => setIsOpen(false)} />}
    </ImportNovelModalContext.Provider>
  );
}

export function ImportNovelModal({ onClose }: { onClose: () => void }) {
  const [manuscript, setManuscript] = useState<string | null>(null);

  return (
    <Modal
      title="导入小说，创建有声剧目"
      description="自动识别章节标记，拆分对白与旁白并建立生产流水线"
      onClose={onClose}
    >
      <form
        onSubmit={(e) => {
          e.preventDefault();
          onClose();
        }}
      >
        <div className={modalBody}>
          <div>
            <label className={fieldLabel} htmlFor="project-title">
              剧目名称
            </label>
            <input
              id="project-title"
              autoFocus
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
                onChange={(e) => setManuscript(e.target.files?.[0]?.name ?? null)}
              />
            </label>
          </div>
        </div>

        <div className={modalFooter}>
          <button type="button" onClick={onClose} className={secondaryButton}>
            取消
          </button>
          <button type="submit" className={primaryButton}>
            开始解析并进入剧目
            <ArrowRight className="h-4 w-4" />
          </button>
        </div>
      </form>
    </Modal>
  );
}
