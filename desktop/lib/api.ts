import { cacheLife, cacheTag } from "next/cache";
import type { ProjectChapter, ProjectStatus, ProjectSummary } from "@/lib/projects";

type ApiEnvelope<T> = {
  code: number;
  message: string;
  data: T;
  request_id: string | null;
};

type ApiProject = {
  project_id: string;
  name: string;
  file_id: string;
  filename: string;
  status: ProjectStatus;
  chapter_count: number;
  total_words: number;
  created_at: string;
  updated_at: string;
};

type ApiProjectList = {
  total: number;
  page: number;
  page_size: number;
  pages: number;
  items: ApiProject[];
};

type ApiChapterOutline = {
  chapter_id: string;
  chapter_index: number;
  chapter_title: string;
  word_count: number;
};

function backendUrl() {
  const base = process.env.API_BASE_URL;
  if (!base) {
    throw new Error(
      "未配置 API_BASE_URL：复制 desktop/.env.example 为 desktop/.env.local 并填写后端地址",
    );
  }
  return base.replace(/\/+$/, "");
}

/** 后端统一响应包装：HTTP 200 但 code 非 0 同样按失败处理 */
async function apiSend<T>(
  method: "GET" | "DELETE",
  path: string,
  params: Record<string, string | number> = {},
): Promise<T> {
  const base = backendUrl();
  const query = new URLSearchParams(
    Object.entries(params).map(([key, value]) => [key, String(value)]),
  );
  let res: Response;
  try {
    res = await fetch(`${base}${path}?${query}`, {
      method,
      headers: { Accept: "application/json" },
    });
  } catch (cause) {
    throw new Error(`无法连接后端 ${base}`, { cause });
  }
  const body = (await res.json().catch(() => null)) as ApiEnvelope<T> | null;

  if (res.status === 404 || body?.code === 40400) {
    return null as T;
  }
  if (!body || body.code !== 0) {
    throw new Error(`${method} ${path} 请求失败：${body?.message ?? `HTTP ${res.status}`}`);
  }
  return body.data;
}

const apiGet = <T>(path: string, params?: Record<string, string | number>) =>
  apiSend<T>("GET", path, params);

function toSummary(item: ApiProject, now: number): ProjectSummary {
  return {
    id: item.project_id,
    title: item.name,
    filename: item.filename,
    chapters: item.chapter_count,
    totalWords: item.total_words,
    // 相对时间在服务端算成小时数后下发，避免水合两端各取当前时刻导致不一致
    editedHoursAgo: Math.max(0, (now - Date.parse(item.updated_at)) / 3_600_000),
  };
}

/** 剧目库列表：GET /api/v1/projects，单页取后端允许的上限 100 条 */
export async function listProjects(): Promise<ProjectSummary[]> {
  // 缓 5 秒：prefetch 与页面内多次导航不再反复打后端，新建/删除后用 revalidateTag("projects") 立即失效
  "use cache";
  cacheLife("seconds");
  cacheTag("projects");

  const data = await apiGet<ApiProjectList>("/api/v1/projects", { page: 1, page_size: 100 });
  const now = Date.now();

  return data.items.map((item) => toSummary(item, now));
}

/** 剧目详情：GET /api/v1/projects/{id}，后端没有该剧目时返回 null */
export async function getProject(id: string): Promise<ProjectSummary | null> {
  "use cache";
  cacheLife("seconds");
  cacheTag("projects");

  const data = await apiGet<ApiProject>(`/api/v1/projects/${id}`);
  return data ? toSummary(data, Date.now()) : null;
}

/** 章节大纲：GET /api/v1/projects/{id}/chapters，按章节序号升序 */
export async function listChapters(id: string): Promise<ProjectChapter[]> {
  "use cache";
  cacheLife("seconds");
  cacheTag("projects");

  const data = await apiGet<{ project_id: string; total: number; items: ApiChapterOutline[] }>(
    `/api/v1/projects/${id}/chapters`,
  );
  if (!data) return [];

  return data.items.map((item) => ({
    id: item.chapter_id,
    index: item.chapter_index,
    title: item.chapter_title,
    words: item.word_count,
  }));
}

/** 删除剧目：DELETE /api/v1/projects/{id}，级联清掉小说、章节与切句；后端无该剧目时返回 null */
export async function deleteProject(id: string): Promise<string | null> {
  const data = await apiSend<{ project_id: string; name: string; deleted: boolean }>(
    "DELETE",
    `/api/v1/projects/${id}`,
  );
  return data?.name ?? null;
}
