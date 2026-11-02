"use server";

import { refresh, updateTag } from "next/cache";
import {
  appendChapters,
  clearTrash,
  createProject,
  deleteProject,
  hardDeleteProject,
  renameProject,
  restoreProject,
} from "@/lib/api";
import type { ChapterAppendResult } from "@/lib/api";
import type { ProjectSummary } from "@/lib/projects";

export type MutationResult = { ok: true } | { ok: false; message: string };
export type ClearTrashResult = { ok: true; count: number } | { ok: false; message: string };
export type CreateProjectResult =
  | { ok: true; project: ProjectSummary }
  | { ok: false; message: string };
export type AppendChaptersResult =
  | { ok: true; preview: ChapterAppendResult }
  | { ok: false; message: string };

/**
 * 回收站相关的写操作都走服务端动作（后端未开 CORS，浏览器不直连）。
 * updateTag 立刻作废服务端缓存，refresh 让浏览器丢掉已缓存的 RSC payload 重新取数，
 * 否则切到另一条路由会看到上次导航/预取留下的旧列表。
 */
function committed(): void {
  updateTag("projects");
  refresh();
}

/** 新建剧目：表单里取名称与 .txt 正文，交给后端清洗分章 */
export async function createProjectAction(formData: FormData): Promise<CreateProjectResult> {
  const name = String(formData.get("name") ?? "").trim();
  const file = formData.get("file");

  if (!name) return { ok: false, message: "请填写剧目名称" };
  if (!(file instanceof File) || file.size === 0) {
    return { ok: false, message: "请选择小说正文文件" };
  }

  let project: ProjectSummary;
  try {
    project = await createProject(name, file);
  } catch (error) {
    return failure(error);
  }

  committed();
  return { ok: true, project };
}

/**
 * 追更上传章节：dryRun 只预检（不动缓存），真正上传后作废缓存并刷新路由，
 * 新章节由 listChapters 重新拉取，因此接口无需返回章节标题清单。
 */
export async function appendChaptersAction(formData: FormData): Promise<AppendChaptersResult> {
  const projectId = String(formData.get("projectId") ?? "");
  const file = formData.get("file");
  const overwriteExisting = formData.get("overwriteExisting") === "true";
  const dryRun = formData.get("dryRun") === "true";

  if (!projectId) return { ok: false, message: "缺少剧目标识" };
  if (!(file instanceof File) || file.size === 0) {
    return { ok: false, message: "请选择章节文件" };
  }

  try {
    const preview = await appendChapters(projectId, file, { overwriteExisting, dryRun });
    if (!dryRun) committed();
    return { ok: true, preview };
  } catch (error) {
    return failure(error);
  }
}

function failure(error: unknown): { ok: false; message: string } {
  return { ok: false, message: error instanceof Error ? error.message : "请求失败" };
}

/** 剧目重命名：先挡掉空名，长度等其余校验交给 PATCH /api/v1/projects/{id} */
export async function renameProjectAction(id: string, title: string): Promise<MutationResult> {
  const name = title.trim();
  if (!name) return { ok: false, message: "剧目名称不能为空" };

  try {
    await renameProject(id, name);
  } catch (error) {
    return failure(error);
  }

  committed();
  return { ok: true };
}

/** 移入回收站（软删除） */
export async function removeProjects(ids: string[]): Promise<MutationResult> {
  try {
    for (const id of ids) await deleteProject(id);
  } catch (error) {
    return failure(error);
  }

  committed();
  return { ok: true };
}

/** 从回收站恢复 */
export async function restoreProjects(ids: string[]): Promise<MutationResult> {
  try {
    for (const id of ids) await restoreProject(id);
  } catch (error) {
    return failure(error);
  }

  committed();
  return { ok: true };
}

/** 彻底删除（物理硬删除，不可逆） */
export async function purgeProjects(ids: string[]): Promise<MutationResult> {
  try {
    for (const id of ids) await hardDeleteProject(id);
  } catch (error) {
    return failure(error);
  }

  committed();
  return { ok: true };
}

/** 清空回收站 */
export async function clearAllTrash(): Promise<ClearTrashResult> {
  try {
    const count = await clearTrash();
    committed();
    return { ok: true, count };
  } catch (error) {
    return failure(error);
  }
}
