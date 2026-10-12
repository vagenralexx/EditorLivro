import { invoke } from "@tauri-apps/api/core";

export interface Sample { before: string; matched: string; after: string; replacement: string; }
export interface ChapterHit { path: string; name: string; count: number; partial: number; forms: Record<string, number>; samples: Sample[]; }
export interface NoteHit { path: string; title: string; kind: string; count: number; entity: boolean; newStem: string | null; canon: boolean; samples: Sample[]; }
export interface Preview { chapters: ChapterHit[]; scanned: number; notes: NoteHit[]; glossaryCount: number; total: number; partialTotal: number; forms: Record<string, number>; partialWords: Record<string, number>; }
export interface Applied { id: string; changedChapters: string[]; replaced: number; notesChanged: string[]; aliasesAdded: string[]; renamed: [string, string][]; glossaryChanged: boolean; warnings: string[]; }
export interface HistoryItem { id: string; createdAt: string; find: string; replace: string; files: number; }
export interface Undone { restored: string[]; changedChapters: string[]; conflicts: string[]; }
export interface ReplaceRequest { root: string; paths: string[]; find: string; replace: string; ignoreAccents: boolean; preserveCase: boolean; }

export const replacePreview = (request: ReplaceRequest) => invoke<Preview>("replace_preview", { request });
export const replaceApply = (request: ReplaceRequest & { chapters: string[]; notes: string[]; aliasNotes: string[]; renameNotes: string[]; glossary: boolean }) => invoke<Applied>("replace_apply", { request });
export const replaceHistory = (root: string) => invoke<HistoryItem[]>("replace_history", { root });
export const replaceUndo = (root: string, id: string, force: boolean) => invoke<Undone>("replace_undo", { root, id, force });

/** Compara caminhos de Windows/Linux ignorando separador, caixa e prefixo `\\?\`. */
export function samePath(a: string, b: string): boolean {
  const norm = (p: string) => p.replace(/^\\\\\?\\/u, "").replace(/\\/gu, "/").toLowerCase();
  const [x, y] = [norm(a), norm(b)];
  return x === y || x.endsWith(`/${y}`) || y.endsWith(`/${x}`);
}
