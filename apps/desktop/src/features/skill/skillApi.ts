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

/** Converte `glossario.json` (formato de producao.md da skill) em linhas para o prompt. */
export function glossaryLines(raw: string): string[] {
  try {
    const data = JSON.parse(raw) as { nao_traduzir?: string[]; traducoes_fixas?: Record<string, string>; personagens?: Record<string, { apelidos?: string[]; tratamentos?: string[] }> };
    const lines: string[] = [];
    if (data.nao_traduzir?.length) lines.push(`NÃO alterar nem traduzir: ${data.nao_traduzir.join(", ")}`);
    for (const [from, to] of Object.entries(data.traducoes_fixas ?? {})) lines.push(`Tradução fixa: “${from}” → “${to}”`);
    for (const [name, info] of Object.entries(data.personagens ?? {})) {
      const extras = [...(info.apelidos ?? []), ...(info.tratamentos ?? [])];
      lines.push(extras.length ? `Personagem ${name} (apelidos/tratamentos: ${extras.join(", ")})` : `Personagem ${name}`);
    }
    return lines.slice(0, 80);
  } catch { return []; }
}
