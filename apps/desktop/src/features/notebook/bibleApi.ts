import { invoke } from "@tauri-apps/api/core";

export type BibleLevel = "serie" | "livro" | "arco";
export type CanonStatus = "proposto" | "confirmado" | "aberto";

export const levelInfo: Record<BibleLevel, { label: string; hint: string }> = {
  serie: { label: "Série", hint: "Visão geral da obra inteira: premissa, linha do tempo macro, regras que valem em todos os livros." },
  livro: { label: "Livro", hint: "Um volume: linha do tempo, idades, personagens, regras e âncoras para o próximo." },
  arco: { label: "Arco", hint: "Um arco que atravessa capítulos ou livros: premissa, marcos, personagens e âncoras." },
};

export interface BibleInfo { path: string; title: string; level: BibleLevel; book: string | null; }
export interface BibleSection { id: string | null; title: string; body: string; status: CanonStatus | null; }
export interface BibleDoc extends BibleInfo { sections: BibleSection[]; }
export interface SectionSpec { id: string; title: string; purpose: string; format: string; questions: string[]; levels: BibleLevel[]; }

export const bibleCatalog = () => invoke<SectionSpec[]>("bible_catalog");
export const bibleList = (root: string) => invoke<BibleInfo[]>("bible_list", { root });
export const bibleCreate = (root: string, level: BibleLevel, title: string, book: string | null) => invoke<string>("bible_create", { root, level, title, book });
export const bibleRead = (root: string, path: string) => invoke<BibleDoc>("bible_read", { root, path });
export const bibleWriteSection = (root: string, path: string, section: string, body: string, status: CanonStatus | null) => invoke<BibleDoc>("bible_write_section", { root, path, section, body, status });
export const chaptersText = (root: string, paths: string[]) => invoke<string[]>("chapters_text", { root, paths });
export const bibleAiDraft = (input: { root: string; path: string; section: string; mode: "respostas" | "manuscrito"; notes?: string; text?: string; current?: string; provider: string; model: string; reasoningEffort: string | null; genre: string; glossary: string[] }) => invoke<string>("bible_ai_draft", { input });

/** Divide o texto dos capítulos em blocos de até `limit` caracteres, sem partir parágrafos. */
export function chunkChapters(texts: { name: string; text: string }[], limit: number): string[] {
  const blocks: string[] = [];
  let current = "";
  const push = (piece: string) => { if (current && current.length + piece.length > limit) { blocks.push(current); current = ""; } current += piece; };
  for (const { name, text } of texts) {
    const header = `\n=== ${name} ===\n`;
    let rest = text;
    push(header);
    while (rest.length > 0) {
      const room = Math.max(limit - header.length, 1000);
      if (rest.length <= room) { push(rest); rest = ""; break; }
      let cut = rest.lastIndexOf("\n", room);
      if (cut < room / 2) cut = room;
      push(rest.slice(0, cut)); blocks.push(current); current = ""; rest = rest.slice(cut).replace(/^\n+/u, "");
      if (rest) current = `\n=== ${name} (continuação) ===\n`;
    }
  }
  if (current.trim()) blocks.push(current);
  return blocks;
}
