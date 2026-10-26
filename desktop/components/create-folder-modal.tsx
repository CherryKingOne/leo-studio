"use client";

import { useState } from "react";
import {
  fieldControl,
  fieldLabel,
  Modal,
  modalBody,
  modalFooter,
  primaryButton,
  secondaryButton,
} from "@/components/modal";

export function CreateFolderModal({
  onClose,
  onCreate,
}: {
  onClose: () => void;
  onCreate: (title: string) => void;
}) {
  const [title, setTitle] = useState("");
  const trimmed = title.trim();

  return (
    <Modal
      title="新建文件夹"
      description="把同一系列或同一作者的剧目归到一起"
      onClose={onClose}
    >
      <form
        onSubmit={(e) => {
          e.preventDefault();
          if (!trimmed) return;
          onCreate(trimmed);
          onClose();
        }}
      >
        <div className={modalBody}>
          <div>
            <label className={fieldLabel} htmlFor="folder-title">
              文件夹名称
            </label>
            <input
              id="folder-title"
              autoFocus
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              placeholder="例如：东方民俗系列"
              className={fieldControl}
            />
          </div>
        </div>

        <div className={modalFooter}>
          <button type="button" onClick={onClose} className={secondaryButton}>
            取消
          </button>
          <button type="submit" disabled={!trimmed} className={primaryButton}>
            创建
          </button>
        </div>
      </form>
    </Modal>
  );
}
