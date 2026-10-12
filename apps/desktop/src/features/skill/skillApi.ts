import { invoke } from "@tauri-apps/api/core";

export type GenreId = "literaria" | "memoir" | "comercial" | "thriller" | "romance" | "sff" | "nao-ficcao";

export interface GenreInfo { id: GenreId; label: string; }
export interface SkillProfile {
  id: string;
  label: string;
  shortLabel: string;
  description: string;
  output: string;
  /** Nível de modelo sugerido pela skill. */
  tier: "Rápido" | "Padrão" | "Máximo";
  suggestedScope: "page" | "chapter";
  /** Só funciona com o texto original carregado (revisão 1:1). */
  needsOriginal: boolean;
  /** Recebe a memória do caderno (recaps e notas) quando ela existe. */
  usesMemory: boolean;
}
export interface PatternInfo { id: number; name: string; detectable: boolean; }
export interface SkillCatalog { genres: GenreInfo[]; profiles: SkillProfile[]; patterns: PatternInfo[]; }

export interface ScanOptions { genre: GenreId; translationMode: boolean; adultContent: boolean; isOpening: boolean; }
export interface PatternHit { id: number; name: string; count: number; examples: string[]; }
export interface ScanNote { kind: string; message: string; example: string | null; }
export interface FluencySuggestion { before: string; after: string; reason: string; }
export interface ScanReport {
  words: number; sentences: number; paragraphs: number;
  avgSentenceWords: number; sentenceVariation: number; hasAcceleration: boolean; hasDeceleration: boolean;
  dialoguePercent: number; dialogueTarget: [number, number] | null;
  tags: { tag: string; count: number }[];
  patterns: PatternHit[]; patternTotal: number; patternsPer3k: number; patternsMax: number; patternsCovered: number;
  notes: ScanNote[]; fluency: FluencySuggestion[];
}

export interface DimensionScore { dimension: string; score: number; evidence: string; }
export interface Genesis { dimensions: DimensionScore[]; floor: number; mean: number; score: number; threshold: number; approved: boolean; }

export const getSkillCatalog = () => invoke<SkillCatalog>("get_skill_catalog");
export const scanEditorial = (text: string, options: ScanOptions) => invoke<ScanReport>("scan_editorial", { text, options });

// ---- Glossário do projeto (glossario.json) ----
export interface GlossaryCharacter { apelidos: string[]; tratamentos: string[]; }
export interface Glossary {
  nao_traduzir: string[];
  traducoes_fixas: Record<string, string>;
  personagens: Record<string, GlossaryCharacter>;
  palavroes: Record<string, string[]>;
  [extra: string]: unknown;
}
export interface TermCandidate { term: string; count: number; chapters: number; kind: "name" | "invented"; }
export const emptyGlossary = (): Glossary => ({ nao_traduzir: [], traducoes_fixas: {}, personagens: {}, palavroes: {} });

export const glossaryLoad = (root: string) => invoke<{ exists: boolean; glossary: Glossary }>("glossary_load", { root });
export const glossarySave = (root: string, glossary: Glossary) => invoke<Glossary>("glossary_save", { root, glossary });
export const glossaryDefaults = () => invoke<Record<string, string[]>>("glossary_defaults");
export const glossarySuggest = (root: string, paths: string[], known: string[]) => invoke<TermCandidate[]>("glossary_suggest", { root, paths, known });

/** Linhas de glossário para o prompt, a partir do objeto carregado. */
export function glossaryToLines(data: Glossary): string[] {
  const lines: string[] = [];
  if (data.nao_traduzir.length) lines.push(`NÃO alterar nem traduzir: ${data.nao_traduzir.join(", ")}`);
  for (const [from, to] of Object.entries(data.traducoes_fixas)) lines.push(`Tradução fixa: “${from}” → “${to}”`);
  for (const [name, info] of Object.entries(data.personagens)) {
    const extras = [...info.apelidos, ...info.tratamentos];
    lines.push(extras.length ? `Personagem ${name} (apelidos/tratamentos: ${extras.join(", ")})` : `Personagem ${name}`);
  }
  for (const [raw, equivalents] of Object.entries(data.palavroes)) lines.push(`Linguagem crua: “${raw}” deve virar ${equivalents.join(" / ")} (sem suavizar)`);
  return lines.slice(0, 100);
}

// ---- Revisão de tradução em duas telas ----
export interface ChapterText { name: string; lines: string[]; }
export interface ParagraphFlag { index: number; kind: "omissao" | "adicao"; ratio: number; original: string; translated: string; }
export interface TermCheck { term: string; original: number; translated: number; status: string; }
export interface RawCheck { raw: string; equivalents: string[]; original: number; translated: number; status: string; }
export interface FidelityReport {
  originalParagraphs: number; translatedParagraphs: number; originalDialogues: number; translatedDialogues: number;
  originalWords: number; translatedWords: number; expansion: number; aligned: boolean;
  flags: ParagraphFlag[]; terms: TermCheck[]; rawLanguage: RawCheck[]; notes: string[];
}
export const originalLoad = (path: string) => invoke<ChapterText[]>("original_load", { path });
export const compareTranslation = (original: string[], translated: string[], glossary: Glossary) => invoke<FidelityReport>("compare_translation", { original, translated, glossary });
