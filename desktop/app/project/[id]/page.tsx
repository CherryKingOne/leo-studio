import type { Metadata } from "next";
import { notFound } from "next/navigation";
import { CloneVoiceModalProvider } from "@/components/clone-voice-modal";
import { HubChaptersProvider } from "@/components/hub-chapters";
import { HubHeader } from "@/components/hub-header";
import { ProjectHub } from "@/components/project-hub";
import { ToastProvider } from "@/components/toast";
import { hubFor, initialProjects } from "@/lib/projects";

// 页面内容按 params 选剧目，属于阻塞式路由
export const instant = false;

function findProject(id: string) {
  return initialProjects.find((project) => project.id === id);
}

export function generateStaticParams() {
  return initialProjects.map((project) => ({ id: project.id }));
}

export async function generateMetadata(
  props: PageProps<"/project/[id]">,
): Promise<Metadata> {
  const { id } = await props.params;
  const project = findProject(id);
  return {
    title: project ? `${project.title} · 剧目看板` : "剧目看板",
  };
}

export default async function ProjectHubPage(props: PageProps<"/project/[id]">) {
  const { id } = await props.params;
  const project = findProject(id);
  if (!project) notFound();

  const hub = hubFor(project);

  return (
    <ToastProvider>
      <CloneVoiceModalProvider>
        <HubChaptersProvider initialChapters={hub.chapters}>
          <div className="flex min-h-screen flex-col">
            <HubHeader project={project} />

            <main className="mx-auto w-full max-w-6xl flex-1 px-5 pb-16 pt-8 sm:px-8">
              <ProjectHub projectId={project.id} cast={hub.cast} />
            </main>
          </div>
        </HubChaptersProvider>
      </CloneVoiceModalProvider>
    </ToastProvider>
  );
}
