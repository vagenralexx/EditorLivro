import { invoke } from "@tauri-apps/api/core";

export interface EpubMetadata {
  title: string;
  authors: string[];
  language: string;
  publisher: string;
  description: string;
  rights: string;
  identifier: string;
  series: string;
  seriesIndex: string;
}

export interface EpubProjectInfo {
  mode: "extracted" | "loose";
  opfPath: string | null;
  epubVersion: string | null;
  metadata: EpubMetadata;
  chapterCount: number;
  hasCover: boolean;
}

export interface EpubIssue { severity: "error" | "warning" | "info"; code: string; file: string | null; message: string; }
export interface EpubReport { issues: EpubIssue[]; errors: number; warnings: number; }
export interface EpubBuildRequest { root: string; metadata: Partial<EpubMetadata>; chapters: string[]; cover: string | null; }
export interface EpubBuildResponse {
  outcome: { path: string; sizeBytes: number; fileCount: number; identifier: string; mode: "extracted" | "loose"; report: EpubReport } | null;
  blockedReport: EpubReport | null;
}

export const emptyEpubMetadata: EpubMetadata = { title: "", authors: [], language: "pt-BR", publisher: "", description: "", rights: "", identifier: "", series: "", seriesIndex: "" };

export const epubInspect = (root: string) => invoke<EpubProjectInfo>("epub_inspect", { root });
export const epubValidate = (request: EpubBuildRequest) => invoke<EpubReport>("epub_validate", { request });
export const epubBuild = (request: EpubBuildRequest, output: string) => invoke<EpubBuildResponse>("epub_build", { request, output });
