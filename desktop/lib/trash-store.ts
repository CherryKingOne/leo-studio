"use client";

import { useSyncExternalStore } from "react";
import type { ProjectSummary } from "@/lib/projects";

/**
 * 回收站条目：后端 DELETE 是物理删除，这里只记录本次浏览器会话内删掉的剧目，
 * restored 表示已被「恢复/撤销」放回剧目库，回收站页不再显示它。
 */
export type TrashItem = {
  project: ProjectSummary;
  deletedAt: number;
  restored: boolean;
};

const storageKey = "leo.project-trash";
const empty: TrashItem[] = [];

let cache: TrashItem[] | null = null;
const listeners = new Set<() => void>();

function emit() {
  for (const listener of listeners) listener();
}

function subscribe(listener: () => void) {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

function load(): TrashItem[] {
  if (cache) return cache;
  try {
    const raw = window.sessionStorage.getItem(storageKey);
    cache = raw ? (JSON.parse(raw) as TrashItem[]) : [];
  } catch {
    cache = [];
  }
  return cache;
}

function save(items: TrashItem[]) {
  cache = items;
  window.sessionStorage.setItem(storageKey, JSON.stringify(items));
  emit();
}

/** 只在客户端读 sessionStorage，服务端快照恒为空，避免水合不一致 */
export function useTrash(): TrashItem[] {
  return useSyncExternalStore(
    subscribe,
    () => (typeof window === "undefined" ? empty : load()),
    () => empty,
  );
}

/** 删除剧目：写进回收站，同一条目只留最新一次记录 */
export function trashProjects(targets: ProjectSummary[]) {
  const now = Date.now();
  const ids = new Set(targets.map((project) => project.id));
  const next = [
    ...targets.map((project) => ({ project, deletedAt: now, restored: false })),
    ...load().filter((item) => !ids.has(item.project.id)),
  ];
  save(next);
  return next;
}

/** 恢复/撤销：标记回剧目库，记录仍留着，方便再次删除时复用 */
export function restoreProjects(ids: Set<string>) {
  save(load().map((item) => (ids.has(item.project.id) ? { ...item, restored: true } : item)));
}

/** 彻底删除：连记录一起抹掉 */
export function purgeProject(id: string) {
  save(load().filter((item) => item.project.id !== id));
}
