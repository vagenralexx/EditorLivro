import { invoke } from "@tauri-apps/api/core";
import { join } from "@tauri-apps/api/path";

/**
 * Estado do projeto guardado NA PASTA (`.livro/projeto.json`), não no navegador do app:
 * reinstalar ou formatar o computador não perde nada enquanto a pasta existir.
 * Chaves em português; cada tela atualiza só a sua seção.
 */
export interface ProjectFile {
  versao?: number;
  configuracoes?: { genero?: string; modoTraducao?: boolean; cenasAdultas?: boolean; usarMemoria?: boolean };
  leitura?: { documento: string; pagina: number };
  original?: { caminho: string };
  epub?: { metadados?: Record<string, unknown>; capa?: string };
  caderno?: { caminho?: string; mapeamento?: Record<string, string> };
  migrado_do_navegador?: boolean;
  [outra: string]: unknown;
}

export interface LoadedProject { file: ProjectFile; /** Mensagem quando o arquivo existe mas está ilegível (nada é gravado nesse caso). */ error: string | null; }

const LEGACY = {
  settings: "book-editor.project-settings.v1",
  positions: "book-editor.project-positions.v1",
  original: "book-editor.original-source.v1",
  epub: "book-editor.epub-export.v1",
};

const cache = new Map<string, ProjectFile>();
const brokenRoots = new Set<string>();

const normalize = (path: string) => path.replace(/\\/gu, "/");

/** Caminho relativo à pasta do projeto (com `/`) quando está dentro dela; senão o absoluto. */
export function toStoredPath(root: string, path: string): string {
  const base = normalize(root).replace(/\/+$/u, "");
  const target = normalize(path);
  return target.toLowerCase().startsWith(`${base.toLowerCase()}/`) ? target.slice(base.length + 1) : path;
}

export async function fromStoredPath(root: string, stored: string): Promise<string> {
  if (/^[a-zA-Z]:[\\/]/u.test(stored) || stored.startsWith("/") || stored.startsWith("\\\\")) return stored;
  return join(root, ...stored.split("/").filter(Boolean));
}

function legacyMap<T>(key: string): Record<string, T> {
  try { return JSON.parse(window.localStorage.getItem(key) ?? "{}") as Record<string, T>; } catch { return {}; }
}
function dropLegacy(key: string, root: string) {
  try { const all = legacyMap<unknown>(key); if (root in all) { delete all[root]; window.localStorage.setItem(key, JSON.stringify(all)); } } catch { /* opcional */ }
}

/** Passa para a pasta o que versões anteriores guardaram no navegador (uma única vez por projeto). */
async function migrateFromBrowser(root: string, file: ProjectFile): Promise<ProjectFile> {
  const next: ProjectFile = { ...file };
  const writes: [string, unknown][] = [];
  const settings = legacyMap<{ genre?: string; translationMode?: boolean; adultContent?: boolean; useMemory?: boolean }>(LEGACY.settings)[root];
  if (settings && !next.configuracoes) writes.push(["configuracoes", { genero: settings.genre, modoTraducao: settings.translationMode, cenasAdultas: settings.adultContent, usarMemoria: settings.useMemory }]);
  const position = legacyMap<{ documentPath?: string; page?: number }>(LEGACY.positions)[root];
  if (position?.documentPath && !next.leitura) writes.push(["leitura", { documento: toStoredPath(root, position.documentPath), pagina: Math.max(1, Math.floor(Number(position.page) || 1)) }]);
  const original = legacyMap<string>(LEGACY.original)[root];
  if (original && !next.original) writes.push(["original", { caminho: toStoredPath(root, original) }]);
  const epub = legacyMap<{ metadata?: Record<string, unknown>; cover?: string }>(LEGACY.epub)[root];
  if (epub && !next.epub) writes.push(["epub", { metadados: epub.metadata ?? {}, capa: epub.cover ? toStoredPath(root, epub.cover) : "" }]);
  if (!writes.length) return next;
  for (const [section, value] of writes) { await invoke("project_state_patch", { root, section, value }); (next as Record<string, unknown>)[section] = value; }
  await invoke("project_state_patch", { root, section: "migrado_do_navegador", value: true });
  next.migrado_do_navegador = true;
  // Só apaga do navegador depois que tudo foi gravado na pasta.
  for (const key of Object.values(LEGACY)) dropLegacy(key, root);
  return next;
}

export async function loadProject(root: string, options: { fresh?: boolean } = {}): Promise<LoadedProject> {
  if (!root) return { file: {}, error: null };
  if (!options.fresh && cache.has(root)) return { file: cache.get(root)!, error: null };
  const loaded = await invoke<{ exists: boolean; state: ProjectFile; error: string | null }>("project_state_load", { root });
  if (loaded.error) { brokenRoots.add(root); return { file: {}, error: loaded.error }; }
  brokenRoots.delete(root);
  let file = loaded.state ?? {};
  try { file = await migrateFromBrowser(root, file); } catch { /* a migração tenta de novo na próxima abertura */ }
  cache.set(root, file);
  return { file, error: null };
}

/** Atualiza uma seção (valor `null` a remove). Não grava se o arquivo do projeto estiver ilegível. */
export async function patchProject(root: string, section: string, value: unknown): Promise<void> {
  if (!root || brokenRoots.has(root)) return;
  await invoke("project_state_patch", { root, section, value });
  const current = { ...(cache.get(root) ?? {}) } as Record<string, unknown>;
  if (value === null) delete current[section]; else current[section] = value;
  cache.set(root, current as ProjectFile);
}

export const isProjectBroken = (root: string) => brokenRoots.has(root);
