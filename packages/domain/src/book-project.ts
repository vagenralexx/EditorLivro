export type ProjectOrigin = "folder" | "epub";

export interface BookProject {
  id: string;
  origin: ProjectOrigin;
  sourcePath: string;
  workspacePath: string;
  title: string;
  createdAt: string;
  updatedAt: string;
}

export interface ChangeSet {
  id: string;
  documentPath: string;
  source: "manual" | "ai" | "bulk";
  createdAt: string;
  summary: string;
  before: string;
  after: string;
}
