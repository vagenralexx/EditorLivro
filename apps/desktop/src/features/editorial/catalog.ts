export type EditorialProfileId =
  | "line-review"
  | "developmental-editor"
  | "translation-review"
  | "story-architect"
  | "brainstorm";

export interface EditorialProfile {
  id: EditorialProfileId;
  label: string;
  shortLabel: string;
  description: string;
  output: string;
  scope: "Seleção" | "Página aberta" | "Bloco de capítulos" | "Livro";
}

export const editorialProfiles: EditorialProfile[] = [
  { id: "line-review", label: "Revisor profissional", shortLabel: "Revisão de linha", description: "Aponta clareza, ritmo, repetições, gramática e preservação de voz sem reescrever em silêncio.", output: "Sugestões em diff, severidade e justificativa editorial.", scope: "Página aberta" },
  { id: "developmental-editor", label: "Editor de desenvolvimento", shortLabel: "Estrutura e capítulo", description: "Avalia função da cena, viradas, tensão, personagens, diálogos, hook e prioridades de revisão.", output: "Diagnóstico estruturado inspirado no pipeline profissional.", scope: "Bloco de capítulos" },
  { id: "translation-review", label: "Revisor de tradução", shortLabel: "Fidelidade 1:1", description: "Compara original e tradução para localizar omissões, tom, glossário, intensidade e fluidez PT-BR.", output: "Pares ancorados e alterações aprováveis; nenhuma substituição cega.", scope: "Página aberta" },
  { id: "story-architect", label: "Arquiteto de narrativa", shortLabel: "Coerência do livro", description: "Cruza bíblia, recap, linha do tempo e fios narrativos para encontrar inconsistências entre capítulos.", output: "Mapa de fios, alertas de continuidade e alternativas respeitosas.", scope: "Livro" },
  { id: "brainstorm", label: "Parceiro de criação", shortLabel: "Brainstorm", description: "Gera alternativas de cena, conflito, arco, gancho ou sinopse sem alterar o manuscrito publicado.", output: "Opções separadas do texto, prontas para comparar e aproveitar.", scope: "Seleção" },
];
