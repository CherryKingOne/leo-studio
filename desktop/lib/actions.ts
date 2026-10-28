"use server";

import { updateTag } from "next/cache";
import { deleteProject } from "@/lib/api";

export type RemoveProjectsResult = {
  ok: true;
  /** 后端确认删除的剧目名，已被他人删掉的不在其中 */
  deleted: string[];
} | { ok: false; message: string };

/**
 * 批量删除剧目走服务端动作：浏览器不直连后端（后端未开 CORS），
 * 全部处理完再失效 projects 缓存标签，列表一次刷新。
 */
export async function removeProjects(ids: string[]): Promise<RemoveProjectsResult> {
  const deleted: string[] = [];

  for (const id of ids) {
    try {
      const name = await deleteProject(id);
      if (name) deleted.push(name);
    } catch (error) {
      return { ok: false, message: error instanceof Error ? error.message : "删除失败" };
    }
  }

  updateTag("projects");
  return { ok: true, deleted };
}
