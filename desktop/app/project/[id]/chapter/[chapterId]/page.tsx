import type { Metadata } from "next";
import { notFound } from "next/navigation";
import { ChapterStudio } from "@/components/chapter-studio";
import { ToastProvider } from "@/components/toast";
import { hubFor, initialEntries, studioFor, type ProjectEntry } from "@/lib/projects";

// 页面内容按 params 选剧目与章节，属于阻塞式路由
export const instant = false;

function findProject(id: string) {
  return initialEntries.find(
    (entry): entry is ProjectEntry => entry.kind === "project" && entry.id === id,
  );
}

function findChapter(projectId: string, chapterId: string) {
  const project = findProject(projectId);
  if (!project) return undefined;
  return hubFor(project).chapters.find((chapter) => chapter.id === chapterId);
}

/** 预渲染看板里可进入工作台的章节，未知 id 走请求时渲染 */
export function generateStaticParams() {
  return initialEntries
    .filter((entry): entry is ProjectEntry => entry.kind === "project")
    .flatMap((project) =>
      hubFor(project)
        .chapters.filter((chapter) => chapter.openable)
        .map((chapter) => ({ id: project.id, chapterId: chapter.id })),
    );
}

export async function generateMetadata(
  props: PageProps<"/project/[id]/chapter/[chapterId]">,
): Promise<Metadata> {
  const { id, chapterId } = await props.params;
  const chapter = findChapter(id, chapterId);
  return {
    title: chapter ? `${chapter.title} · 制作工作台` : "章节制作工作台",
  };
}

export default async function ChapterStudioPage(
  props: PageProps<"/project/[id]/chapter/[chapterId]">,
) {
  const { id, chapterId } = await props.params;
  const project = findProject(id);
  const chapter = project && hubFor(project).chapters.find((item) => item.id === chapterId);
  if (!project || !chapter) notFound();

  return (
    <ToastProvider>
      <ChapterStudio project={project} chapter={chapter} studio={studioFor(project, chapter)} />
    </ToastProvider>
  );
}
