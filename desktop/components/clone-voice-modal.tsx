"use client";

import { Upload } from "lucide-react";
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
import { useToast } from "@/components/toast";

type CloneVoiceModalValue = { open: () => void };

const CloneVoiceModalContext = createContext<CloneVoiceModalValue | null>(null);

export function useCloneVoiceModal() {
  const value = useContext(CloneVoiceModalContext);
  if (!value) {
    throw new Error("useCloneVoiceModal 必须在 CloneVoiceModalProvider 内使用");
  }
  return value;
}

export function CloneVoiceModalProvider({ children }: { children: ReactNode }) {
  const [isOpen, setIsOpen] = useState(false);

  return (
    <CloneVoiceModalContext.Provider value={{ open: () => setIsOpen(true) }}>
      {children}
      {isOpen && <CloneVoiceModal onClose={() => setIsOpen(false)} />}
    </CloneVoiceModalContext.Provider>
  );
}

export function CloneVoiceModal({ onClose }: { onClose: () => void }) {
  const { show } = useToast();
  const [name, setName] = useState("");
  const [sample, setSample] = useState<string | null>(null);
  const trimmed = name.trim();

  return (
    <Modal
      title="克隆新音色加入本书"
      description="上传 10–20 秒干净人声样本，零样本克隆后可直接绑定角色"
      onClose={onClose}
    >
      <form
        onSubmit={(e) => {
          e.preventDefault();
          onClose();
          show("已开始音色克隆，预计 30 秒完成", "success");
        }}
      >
        <div className={modalBody}>
          <div>
            <label className={fieldLabel} htmlFor="voice-name">
              音色名称
            </label>
            <input
              id="voice-name"
              autoFocus
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="例如：英伦青年音"
              className={fieldControl}
            />
          </div>

          <div>
            <span className={fieldLabel}>人声样本（.wav / .mp3）</span>
            <label className="block cursor-pointer rounded-xl border border-dashed border-white/12 bg-ink-900/60 px-6 py-8 text-center transition-colors hover:border-accent/50">
              <Upload className="mx-auto mb-2.5 h-6 w-6 text-white/30" />
              <div className="text-[13px] text-white/75">{sample ?? "点击或拖拽音频到此处"}</div>
              <div className="mt-1 text-[11px] text-white/35">
                单条最大 10 MB · 建议安静环境录制，无 BGM 干扰
              </div>
              <input
                type="file"
                accept=".wav,.mp3"
                className="sr-only"
                onChange={(e) => setSample(e.target.files?.[0]?.name ?? null)}
              />
            </label>
          </div>
        </div>

        <div className={modalFooter}>
          <button type="button" onClick={onClose} className={secondaryButton}>
            取消
          </button>
          <button type="submit" disabled={!trimmed} className={primaryButton}>
            开始克隆
          </button>
        </div>
      </form>
    </Modal>
  );
}
