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
export interface WatermarkOptions { recipient: string; visibleNotice: boolean; forensic: boolean; }
export interface EpubBuildRequest { root: string; metadata: Partial<EpubMetadata>; chapters: string[]; cover: string | null; watermark?: WatermarkOptions | null; }
export interface WatermarkInfo { token: string; recipient: string; forensicMarkers: number; }
export interface CopyRecord { token: string; recipient: string; title: string; createdAt: string; file: string; }
export interface WatermarkDetection { token: string; markers: number; agreeing: number; }
export interface EpubBuildResponse {
  outcome: { path: string; sizeBytes: number; fileCount: number; identifier: string; mode: "extracted" | "loose"; report: EpubReport; watermark: WatermarkInfo | null } | null;
  blockedReport: EpubReport | null;
}

export const emptyEpubMetadata: EpubMetadata = { title: "", authors: [], language: "pt-BR", publisher: "", description: "", rights: "", identifier: "", series: "", seriesIndex: "" };

export const epubInspect = (root: string) => invoke<EpubProjectInfo>("epub_inspect", { root });
export const epubValidate = (request: EpubBuildRequest) => invoke<EpubReport>("epub_validate", { request });
export const epubBuild = (request: EpubBuildRequest, output: string) => invoke<EpubBuildResponse>("epub_build", { request, output });
export const copiesList = (root: string) => invoke<CopyRecord[]>("copies_list", { root });
export const copiesRegister = (root: string, record: CopyRecord) => invoke<CopyRecord[]>("copies_register", { root, record });
export const detectWatermark = (path: string) => invoke<WatermarkDetection | null>("epub_detect_watermark", { path });
