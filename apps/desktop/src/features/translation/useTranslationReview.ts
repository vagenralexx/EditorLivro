import { open } from "@tauri-apps/plugin-dialog";
import { useCallback, useEffect, useMemo, useState } from "react";
import { compareTranslation, emptyGlossary, originalLoad, type ChapterText, type FidelityReport, type Glossary } from "../skill/skillApi";

const STORAGE_KEY = "book-editor.original-source.v1";

function storedSource(root: string): string {
  try { return (JSON.parse(window.localStorage.getItem(STORAGE_KEY) ?? "{}") as Record<string, string>)[root] ?? ""; } catch { return ""; }
}
function rememberSource(root: string, path: string) {
  try { const all = JSON.parse(window.localStorage.getItem(STORAGE_KEY) ?? "{}") as Record<string, string>; if (path) all[root] = path; else delete all[root]; window.localStorage.setItem(STORAGE_KEY, JSON.stringify(all)); } catch { /* opcional */ }
}

const stem = (name: string) => name.replace(/\.[^.]+$/u, "").toLowerCase();

/** Trecho do original proporcional à página aberta, com folga de meia página nas pontas. */
export function originalSliceForPage(lines: string[], page: number, pageCount: number): string[] {
  if (pageCount <= 1) return lines;
  const per = lines.length / pageCount;
  const start = Math.max(0, Math.floor((page - 1) * per - per * 0.5));
  const end = Math.min(lines.length, Math.ceil(page * per + per * 0.5));
  return lines.slice(start, end);
}

interface Params {
  enabled: boolean;
  projectRoot: string;
  activeName: string | undefined;
  activeIndex: number;
  translatedLines: string[];
  glossary: Glossary | null;
}

export function useTranslationReview({ enabled, projectRoot, activeName, activeIndex, translatedLines, glossary }: Params) {
  const [sourcePath, setSourcePath] = useState("");
  const [chapters, setChapters] = useState<ChapterText[]>([]);
  const [manualIndex, setManualIndex] = useState<number | null>(null);
  const [report, setReport] = useState<FidelityReport | null>(null);
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(false);

  const load = useCallback(async (path: string) => {
    setLoading(true); setError("");
    try {
      const loaded = await originalLoad(path);
      setChapters(loaded); setSourcePath(path); setManualIndex(null); rememberSource(projectRoot, path);
    } catch (reason) { setError(String(reason)); } finally { setLoading(false); }
  }, [projectRoot]);

  // Reabre o original usado da última vez neste projeto.
  useEffect(() => {
    setChapters([]); setSourcePath(""); setReport(null); setError("");
    const saved = projectRoot ? storedSource(projectRoot) : "";
    if (enabled && saved) void load(saved);
  }, [projectRoot, enabled, load]);

  const choose = useCallback(async (kind: "epub" | "folder") => {
    const selected = await open(kind === "epub"
      ? { multiple: false, title: "Escolha o EPUB original", filters: [{ name: "EPUB", extensions: ["epub"] }] }
      : { multiple: false, directory: true, title: "Escolha a pasta com os capítulos originais" });
    if (typeof selected === "string") await load(selected);
  }, [load]);

  const clear = useCallback(() => { setChapters([]); setSourcePath(""); setReport(null); rememberSource(projectRoot, ""); }, [projectRoot]);

  // Capítulo do original: mesmo nome (sem extensão) ou, na falta, mesma posição na lista.
  const autoIndex = useMemo(() => {
    if (!chapters.length) return -1;
    const byName = activeName ? chapters.findIndex((chapter) => stem(chapter.name) === stem(activeName)) : -1;
    return byName >= 0 ? byName : Math.min(Math.max(activeIndex, 0), chapters.length - 1);
  }, [activeIndex, activeName, chapters]);
  useEffect(() => { setManualIndex(null); }, [activeName]);
  const index = manualIndex ?? autoIndex;
  const lines = index >= 0 ? chapters[index]?.lines ?? [] : [];

  useEffect(() => {
    if (!enabled || !lines.length || !translatedLines.length) { setReport(null); return undefined; }
    let active = true;
    const timer = window.setTimeout(() => { compareTranslation(lines, translatedLines, glossary ?? emptyGlossary()).then((result) => active && setReport(result)).catch(() => active && setReport(null)); }, 500);
    return () => { active = false; window.clearTimeout(timer); };
  }, [enabled, glossary, lines, translatedLines]);

  return { sourcePath, chapters, index, setIndex: setManualIndex, isAuto: manualIndex === null, lines, report, error, loading, choose, clear, hasOriginal: chapters.length > 0 };
}
