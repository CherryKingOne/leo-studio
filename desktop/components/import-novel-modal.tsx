"use client";

import { ArrowRight, Upload } from "lucide-react";
import { useRouter } from "next/navigation";
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
import { createProjectAction } from "@/lib/actions";

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
  const router = useRouter();
  const [name, setName] = useState("");
  const [manuscript, setManuscript] = useState<File | null>(null);
  const [dragging, setDragging] = useState(false);
  const [pending, setPending] = useState(false);
  const [failure, setFailure] = useState<string | null>(null);

  const ready = name.trim() !== "" && manuscript !== null;

  function pick(list: FileList | null) {
    const file = list?.[0] ?? null;
    if (!file) return;
    if (!/\.txt$/i.test(file.name)) {
      setFailure("后端目前只接受 .txt 正文，请换文件");
      return;
    }
    setFailure(null);
    setManuscript(file);
  }

  async function submit() {
    if (!ready || pending || !manuscript) return;

    setPending(true);
    setFailure(null);

    const form = new FormData();
    form.set("name", name.trim());
    form.set("file", manuscript, manuscript.name);

    const result = await createProjectAction(form);
    if (!result.ok) {
      setFailure(result.message);
      setPending(false);
      return;
    }
    onClose();
    router.push(`/project/${result.project.id}`);
  }

  return (
    <Modal
      title="导入小说，创建有声剧目"
      description="自动识别章节标记，拆分对白与旁白并建立生产流水线"
      onClose={onClose}
    >
      <div className={modalBody}>
        <div>
          <label className={fieldLabel} htmlFor="project-name">
            剧目名称
          </label>
          <input
            id="project-name"
            autoFocus
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="例如：诡秘之主：第一部 愚者"
            className={fieldControl}
          />
        </div>

        <div>
          <span className={fieldLabel}>小说正文（.txt）</span>
          <div
            role="button"
            tabIndex={0}
            onClick={() => document.getElementById("project-file")?.click()}
            onKeyDown={(e) => {
              if (e.key === "Enter" || e.key === " ") document.getElementById("project-file")?.click();
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
            <Upload className="mx-auto mb-2.5 h-6 w-6 text-white/30" />
            <div className="text-[13px] text-white/75">
              {manuscript ? `${manuscript.name} · ${(manuscript.size / 1024 / 1024).toFixed(2)} MB` : "点击选择，或把文件拖拽到这里"}
            </div>
            <div className="mt-1 text-[11px] text-white/35">
              单文件最大 50 MB · 章节名以「第X章」开头最易自动识别
            </div>
          </div>
          <input
            id="project-file"
            type="file"
            accept=".txt"
            className="hidden"
            onChange={(e) => {
              pick(e.target.files);
              e.target.value = "";
            }}
          />
        </div>

        {failure && <p className="text-[12.5px] text-red-400">{failure}</p>}
      </div>

      <div className={modalFooter}>
        <button type="button" onClick={onClose} className={secondaryButton}>
          取消
        </button>
        <button
          type="button"
          disabled={!ready || pending}
          onClick={() => void submit()}
          className={primaryButton}
        >
          <ArrowRight className="h-4 w-4" />
          <span>{pending ? "解析中" : "开始解析并进入剧目"}</span>
        </button>
      </div>
    </Modal>
  );
}
