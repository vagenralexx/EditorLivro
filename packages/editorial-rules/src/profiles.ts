export type EditorialProfile = "line-review" | "developmental-editor" | "translation-review" | "story-architect" | "brainstorm";

export interface EditorialRequest {
  profile: EditorialProfile;
  scope: "selection" | "document" | "block" | "book";
  language: "pt-BR" | "en" | "other";
  text: string;
  projectContext?: { genre?: string; pointOfView?: string; glossary?: Record<string, string>; recap?: string };
}

export interface EditorialSuggestion { severity: "critical" | "important" | "attention" | "preserve"; anchor: string; rationale: string; before?: string; after?: string; }
export interface EditorialResult { summary: string; suggestions: EditorialSuggestion[]; report?: Record<string, unknown>; }
