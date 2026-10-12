import { invoke } from "@tauri-apps/api/core";
import type { NoteKind } from "../notebook/notebookApi";

export interface Reference { path: string; title: string; heading: string | null; kind: NoteKind; count: number; }
export interface LinkRef { path: string; title: string; heading: string | null; kind: NoteKind; }
export interface RecordMention { id: string; note: string; line: number; status: "aberto" | "resolvido" | "supersedido" | null; title: string; snippet: string; }
export interface SectionMention { note: string; noteTitle: string; heading: string; snippet: string; status: string | null; }
export interface NoteMention { path: string; title: string; heading: string | null; kind: NoteKind; count: number; }
export interface EntityContext { path: string; title: string; heading: string | null; kind: NoteKind; aliases: string[]; content: string; outgoing: LinkRef[]; backlinks: LinkRef[]; records: RecordMention[]; sections: SectionMention[]; notes: NoteMention[]; }

export const vaultReferences = (root: string, lines: string[]) => invoke<Reference[]>("vault_references", { root, lines });
export const vaultEntity = (root: string, path: string) => invoke<EntityContext>("vault_entity", { root, path });
