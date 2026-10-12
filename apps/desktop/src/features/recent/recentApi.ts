import { invoke } from "@tauri-apps/api/core";

export interface RecentProject {
  path: string;
  name: string;
  openedAt: string;
  /** A pasta ainda existe (movida ou apagada = false). */
  exists: boolean;
  /** Capítulo (relativo à pasta) e página em que a leitura parou. */
  document: string | null;
  page: number | null;
}

export const recentProjects = () => invoke<RecentProject[]>("recent_projects");
export const touchRecentProject = (path: string) => invoke<RecentProject[]>("recent_projects_touch", { path });

/** "capitulo3.xhtml · página 12" para a dica do botão. */
export function resumeLabel(project: RecentProject): string {
  const doc = project.document?.split("/").pop();
  return [doc, project.page ? `página ${project.page}` : null].filter(Boolean).join(" · ");
}
