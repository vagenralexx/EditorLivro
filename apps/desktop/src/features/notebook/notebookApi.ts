import { invoke } from "@tauri-apps/api/core";

export type NoteKind = "personagem" | "lugar" | "fio" | "evento" | "mundo" | "capitulo" | "nota";

export const kindInfo: Record<NoteKind, { label: string; plural: string; color: string }> = {
  personagem: { label: "Personagem", plural: "Personagens", color: "#2f7fd1" },
  lugar: { label: "Lugar", plural: "Lugares", color: "#2f9e6e" },
  fio: { label: "Fio narrativo", plural: "Fios narrativos", color: "#d1802f" },
  evento: { label: "Evento", plural: "Eventos", color: "#a35bd1" },
  mundo: { label: "Regra do mundo", plural: "Mundo", color: "#c24a5a" },
  capitulo: { label: "Recap de capítulo", plural: "Capítulos (recaps)", color: "#6b7a90" },
  nota: { label: "Nota", plural: "Notas", color: "#8a8f3a" },
};
export const kindOrder: NoteKind[] = ["personagem", "lugar", "fio", "evento", "mundo", "capitulo", "nota"];

export interface NoteMeta { path: string; title: string; kind: NoteKind; aliases: string[]; words: number; modified: number; state: string | null; }
export interface VaultIndex { exists: boolean; notes: NoteMeta[]; edges: { from: string; to: string }[]; unresolved: { from: string; target: string }[]; }
export interface NoteMentions { path: string; chapters: { chapter: number; count: number }[]; total: number; }
export interface TimelineEntry { chapter: string; order: number; events: string[]; }
export interface Recap {
  summary: string;
  timeline: { event: string; scene: string }[];
  characters: { name: string; state: string }[];
  places: { name: string; note: string }[];
  threads_open: { title: string; text: string }[];
  threads_closed: { title: string; text: string }[];
  facts: string[];
  alerts: string[];
}
export interface MemoryContext { text: string; notesUsed: string[]; recapsUsed: string[]; }
export interface RecapChange { path: string; action: string; }

export const vaultIndex = (root: string) => invoke<VaultIndex>("vault_index", { root });
export const vaultRead = (root: string, path: string) => invoke<string>("vault_read", { root, path });
export const vaultWrite = (root: string, path: string, content: string) => invoke<void>("vault_write", { root, path, content });
export const vaultCreate = (root: string, kind: NoteKind, title: string) => invoke<string>("vault_create", { root, kind, title });
export const vaultDelete = (root: string, path: string) => invoke<void>("vault_delete", { root, path });
export const vaultRename = (root: string, path: string, newTitle: string) => invoke<{ newPath: string; updatedNotes: number }>("vault_rename", { root, path, newTitle });
export const vaultTimeline = (root: string) => invoke<TimelineEntry[]>("vault_timeline", { root });
export const vaultMentions = (root: string, paths: string[]) => invoke<NoteMentions[]>("vault_mentions", { root, paths });
export const vaultMemory = (root: string, current: string[], previousChapters: string[]) => invoke<MemoryContext>("vault_memory", { root, current, previousChapters });
export const vaultApplyRecap = (root: string, chapter: string, order: number, recap: Recap) => invoke<RecapChange[]>("vault_apply_recap", { root, chapter, order, recap });

/** Separa o frontmatter YAML do corpo da nota. */
export function splitFrontmatter(text: string): { front: string; body: string } {
  const match = /^---\r?\n([\s\S]*?)\r?\n---\r?\n?/u.exec(text);
  return match ? { front: match[1], body: text.slice(match[0].length) } : { front: "", body: text };
}

export function wikilinks(text: string): string[] {
  return [...text.matchAll(/\[\[([^\[\]\n|#]+)(?:#[^\[\]\n|]*)?(?:\|[^\[\]\n]*)?\]\]/gu)].map((match) => match[1].trim());
}
