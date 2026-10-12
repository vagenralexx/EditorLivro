import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { join } from "@tauri-apps/api/path";
import { message, open } from "@tauri-apps/plugin-dialog";
import { readDir, readTextFile } from "@tauri-apps/plugin-fs";
import { useCallback, useEffect, useMemo, useState } from "react";
import { emptyRecord, friendlyModelName, providerFor, providerIds, providers, type ProviderId } from "../ai/providers";
import { paragraphTextFromXhtml, textFromXhtml } from "../editor/xhtmlDocument";
import { VisualXhtmlEditor, type PaginationState, type ReaderTheme } from "../editor/VisualXhtmlEditor";
import type { EditorialProfileId } from "../editorial/catalog";
import { NotebookDialog } from "../notebook/NotebookDialog";
import { vaultMemory } from "../notebook/notebookApi";
import { FidelityCard } from "../translation/FidelityCard";
import { OriginalPane } from "../translation/OriginalPane";
import { originalSliceForPage, useTranslationReview } from "../translation/useTranslationReview";
import { GenesisCard } from "../skill/GenesisCard";
import { GlossaryDialog } from "../skill/GlossaryDialog";
import { ScanPanel } from "../skill/ScanPanel";
import { getSkillCatalog, glossaryLoad, glossaryToLines, type Glossary, type GenreId, type Genesis, type ScanOptions, type SkillCatalog } from "../skill/skillApi";
import { ExportEpubDialog } from "../export/ExportEpubDialog";
import { AiSettingsDialog } from "../settings/AiSettingsDialog";
import "./WorkspaceDashboard.css";

interface AppBootstrap { productName: string; architecture: string; modules: string[]; }
interface ProjectDocument { name: string; path: string; }
interface EditorialSuggestion { before: string; after: string; reason: string; }
interface EditorialFinding { severity: "baixa" | "média" | "alta" | string; title: string; detail: string; anchor: string; }
interface AnalysisResult { provider: string; model: string; summary: string; findings: EditorialFinding[]; suggestions: EditorialSuggestion[]; genesis: Genesis | null; }
interface ProjectSettings { genre: GenreId; translationMode: boolean; adultContent: boolean; useMemory: boolean; }
const PROJECT_SETTINGS_KEY = "book-editor.project-settings.v1";
const defaultProjectSettings: ProjectSettings = { genre: "comercial", translationMode: false, adultContent: false, useMemory: true };

function loadProjectSettings(root: string): ProjectSettings {
  try { return { ...defaultProjectSettings, ...((JSON.parse(window.localStorage.getItem(PROJECT_SETTINGS_KEY) ?? "{}") as Record<string, Partial<ProjectSettings>>)[root] ?? {}) }; } catch { return defaultProjectSettings; }
}

function saveProjectSettings(root: string, settings: ProjectSettings) {
  if (!root) return;
  try { const all = JSON.parse(window.localStorage.getItem(PROJECT_SETTINGS_KEY) ?? "{}") as Record<string, ProjectSettings>; all[root] = settings; window.localStorage.setItem(PROJECT_SETTINGS_KEY, JSON.stringify(all)); } catch { /* opcional */ }
}
interface AiProgress { elapsedMs: number; answerChars: number; reasoningChars: number; phase: "waiting" | "thinking" | "writing"; }
interface SaveResult { backupPath: string; }
interface EffortOption { value: string; label: string; }
interface ModelEffortOptions { configurable: boolean; levels: EffortOption[]; default: string; note: string; }

const initialModels = (): Record<ProviderId, string> => emptyRecord(() => "");
const initialAvailableModels = (): Record<ProviderId, string[]> => emptyRecord<string[]>(() => []);
const initialReasoningEfforts = (): Record<ProviderId, Record<string, string>> => emptyRecord<Record<string, string>>(() => ({}));
const AI_PREFERENCES_KEY = "book-editor.ai-preferences.v1";
const PROJECT_POSITIONS_KEY = "book-editor.project-positions.v1";

interface PersistedAiPreferences {
  providerId?: ProviderId;
  models?: Record<ProviderId, string>;
  availableModels?: Record<ProviderId, string[]>;
  validatedProviders?: Partial<Record<ProviderId, boolean>>;
  reasoningEfforts?: Record<ProviderId, Record<string, string>>;
}

interface ProjectPosition { documentPath: string; page: number; }

function loadProjectPositions(): Record<string, ProjectPosition> {
  try {
    const parsed = JSON.parse(window.localStorage.getItem(PROJECT_POSITIONS_KEY) ?? "{}") as Record<string, Partial<ProjectPosition>>;
    return Object.fromEntries(Object.entries(parsed).flatMap(([root, position]) => {
      const page = Number(position.page);
      return typeof position.documentPath === "string" && position.documentPath && Number.isFinite(page) && page > 0
        ? [[root, { documentPath: position.documentPath, page: Math.floor(page) }]] : [];
    }));
  } catch { return {}; }
}

function saveProjectPosition(projectRoot: string, documentPath: string, page: number) {
  if (!projectRoot || !documentPath || page < 1) return;
  const positions = loadProjectPositions();
  positions[projectRoot] = { documentPath, page };
  window.localStorage.setItem(PROJECT_POSITIONS_KEY, JSON.stringify(positions));
}

function loadAiPreferences(): PersistedAiPreferences {
  try {
    const parsed = JSON.parse(window.localStorage.getItem(AI_PREFERENCES_KEY) ?? "{}") as PersistedAiPreferences;
    const efforts = initialReasoningEfforts();
    for (const id of providerIds) efforts[id] = { ...(parsed.reasoningEfforts?.[id] ?? {}) };
    return {
      providerId: providerIds.includes(parsed.providerId as ProviderId) ? parsed.providerId : "nvidia",
      models: { ...initialModels(), ...parsed.models },
      availableModels: { ...initialAvailableModels(), ...parsed.availableModels },
      validatedProviders: parsed.validatedProviders ?? {},
      reasoningEfforts: efforts,
    };
  } catch { return { providerId: "nvidia", models: initialModels(), availableModels: initialAvailableModels(), validatedProviders: {}, reasoningEfforts: initialReasoningEfforts() }; }
}

function progressLabel(progress: AiProgress | null) {
  if (!progress) return "Enviando ao servidor…";
  const seconds = Math.round(progress.elapsedMs / 1000);
  if (progress.phase === "writing") return `Escrevendo a análise… ${progress.answerChars} caracteres · ${seconds} s`;
  if (progress.phase === "thinking") return `Modelo raciocinando… ${seconds} s`;
  return `Aguardando o servidor (fila)… ${seconds} s`;
}

function isXhtml(path: string) { return /\.x?html$/iu.test(path); }
const chapterCollator = new Intl.Collator("pt-BR", { numeric: true, sensitivity: "base" });

async function collectXhtmlFiles(directory: string): Promise<ProjectDocument[]> {
  const entries = await readDir(directory);
  const nested = await Promise.all(entries.map(async (entry) => {
    const path = await join(directory, entry.name);
    if (entry.isDirectory) return collectXhtmlFiles(path);
    return entry.isFile && isXhtml(entry.name) ? [{ name: entry.name, path }] : [];
  }));
  return nested.flat().sort((left, right) => chapterCollator.compare(left.path, right.path));
}

function fileNameFromPath(path: string) {
  const parts = path.split(/[\\/]/u).filter(Boolean);
  return parts[parts.length - 1] ?? path;
}

export function WorkspaceDashboard() {
  const [savedAiPreferences] = useState(loadAiPreferences);
  const [providerId, setProviderId] = useState<ProviderId>(() => savedAiPreferences.providerId ?? "nvidia");
  const [modelQuery, setModelQuery] = useState("");
  const [models, setModels] = useState(() => savedAiPreferences.models ?? initialModels());
  const [availableModels, setAvailableModels] = useState(() => savedAiPreferences.availableModels ?? initialAvailableModels());
  const [reasoningEfforts, setReasoningEfforts] = useState<Record<ProviderId, Record<string, string>>>(() => savedAiPreferences.reasoningEfforts ?? initialReasoningEfforts());
  const [effortOptions, setEffortOptions] = useState<ModelEffortOptions | null>(null);
  const [profileId, setProfileId] = useState<EditorialProfileId>("line-review");
  const [catalog, setCatalog] = useState<SkillCatalog | null>(null);
  const [projectSettings, setProjectSettings] = useState<ProjectSettings>(defaultProjectSettings);
  const [scope, setScope] = useState<"page" | "chapter">("page");
  const [glossary, setGlossary] = useState<string[]>([]);
  const [isGlossaryOpen, setIsGlossaryOpen] = useState(false);
  const [isNotebookOpen, setIsNotebookOpen] = useState(false);
  const [memoryNote, setMemoryNote] = useState("");
  const [glossaryData, setGlossaryData] = useState<Glossary | null>(null);
  const [splitView, setSplitView] = useState(true);
  const [genesis, setGenesis] = useState<Genesis | null>(null);
  const [bootstrap, setBootstrap] = useState<AppBootstrap | null>(null);
  const [documents, setDocuments] = useState<ProjectDocument[]>([]);
  const [activeDocument, setActiveDocument] = useState<ProjectDocument | null>(null);
  const [editorText, setEditorText] = useState("");
  const [savedText, setSavedText] = useState("");
  const [projectRoot, setProjectRoot] = useState("");
  const [isOpening, setIsOpening] = useState(false);
  const [isSettingsOpen, setIsSettingsOpen] = useState(false);
  const [isExportOpen, setIsExportOpen] = useState(false);
  const [keyStatus, setKeyStatus] = useState<Partial<Record<ProviderId, boolean>>>(() => savedAiPreferences.validatedProviders ?? {});
  const [analysis, setAnalysis] = useState("");
  const [findings, setFindings] = useState<EditorialFinding[]>([]);
  const [suggestions, setSuggestions] = useState<EditorialSuggestion[]>([]);
  const [analysisError, setAnalysisError] = useState("");
  const [isAnalyzing, setIsAnalyzing] = useState(false);
  const [aiProgress, setAiProgress] = useState<AiProgress | null>(null);
  const [isSaving, setIsSaving] = useState(false);
  const [readerTheme, setReaderTheme] = useState<ReaderTheme>("warm");
  const [isAppDark, setIsAppDark] = useState(false);
  const [fontScale, setFontScale] = useState(100);
  const [brightness, setBrightness] = useState(100);
  const [pagination, setPagination] = useState<PaginationState>({ page: 1, pageCount: 1, xhtml: "" });
  const [resumePage, setResumePage] = useState(1);
  const [externalEditorRevision, setExternalEditorRevision] = useState(0);
  const [pendingDocument, setPendingDocument] = useState<ProjectDocument | null>(null);

  useEffect(() => { invoke<AppBootstrap>("get_app_bootstrap").then(setBootstrap).catch(() => setBootstrap(null)); }, []);

  useEffect(() => {
    window.localStorage.setItem(AI_PREFERENCES_KEY, JSON.stringify({ providerId, models, availableModels, validatedProviders: keyStatus, reasoningEfforts }));
  }, [availableModels, keyStatus, models, providerId, reasoningEfforts]);

  useEffect(() => {
    let active = true;
    Promise.all(providerIds.map(async (provider) => [provider, await invoke<boolean>("has_provider_api_key", { provider })] as const))
      .then((entries) => {
        if (!active) return;
        setKeyStatus((current) => ({ ...current, ...Object.fromEntries(entries) }));
        setAvailableModels((current) => {
          const next = { ...current };
          for (const [provider, exists] of entries) if (!exists) next[provider] = [];
          return next;
        });
      })
      .catch(() => undefined);
    return () => { active = false; };
  }, []);

  // Progresso ao vivo da análise (fila → raciocínio → escrita), enviado pelo backend.
  useEffect(() => {
    if (!isAnalyzing) { setAiProgress(null); return undefined; }
    let unlisten: (() => void) | undefined;
    let active = true;
    listen<AiProgress>("ai-progress", (event) => setAiProgress(event.payload)).then((stop) => { if (active) unlisten = stop; else stop(); }).catch(() => undefined);
    return () => { active = false; unlisten?.(); };
  }, [isAnalyzing]);

  const provider = useMemo(() => providerFor(providerId), [providerId]);
  // Catálogo unificado: só provedores com chave salva e modelos carregados, agrupados por servidor.
  const modelGroups = useMemo(() => {
    const query = modelQuery.trim().toLocaleLowerCase("pt-BR");
    return providers.flatMap((item) => {
      if (!keyStatus[item.id]) return [];
      const entries = availableModels[item.id]
        .filter((model) => !query || `${friendlyModelName(model)} ${model} ${item.label}`.toLocaleLowerCase("pt-BR").includes(query))
        .sort((left, right) => friendlyModelName(left).localeCompare(friendlyModelName(right), "pt-BR"));
      return entries.length ? [{ provider: item, models: entries }] : [];
    });
  }, [availableModels, keyStatus, modelQuery]);
  const hasAnyCatalog = providers.some((item) => keyStatus[item.id] && availableModels[item.id].length > 0);
  const selectedModelKey = models[providerId] && modelGroups.some((group) => group.provider.id === providerId && group.models.includes(models[providerId])) ? `${providerId}::${models[providerId]}` : "";
  const chooseModel = useCallback((value: string) => {
    const separator = value.indexOf("::");
    if (separator < 0) return;
    const id = value.slice(0, separator) as ProviderId;
    if (!providerIds.includes(id)) return;
    setProviderId(id);
    setModels((current) => ({ ...current, [id]: value.slice(separator + 2) }));
  }, []);
  const profile = useMemo(() => catalog?.profiles.find((item) => item.id === profileId) ?? catalog?.profiles[0] ?? null, [catalog, profileId]);
  const genreLabel = useMemo(() => catalog?.genres.find((item) => item.id === projectSettings.genre)?.label ?? "Ficção comercial", [catalog, projectSettings.genre]);
  useEffect(() => { getSkillCatalog().then(setCatalog).catch(() => setCatalog(null)); }, []);
  useEffect(() => { saveProjectSettings(projectRoot, projectSettings); }, [projectRoot, projectSettings]);
  const changeProfile = useCallback((id: EditorialProfileId) => {
    setProfileId(id);
    const next = catalog?.profiles.find((item) => item.id === id);
    if (next) setScope(next.suggestedScope);
  }, [catalog]);
  const chapterText = useMemo(() => (editorText ? paragraphTextFromXhtml(editorText) : ""), [editorText]);
  const scanOptions = useMemo<ScanOptions>(() => ({ genre: projectSettings.genre, translationMode: projectSettings.translationMode, adultContent: projectSettings.adultContent, isOpening: true }), [projectSettings]);
  const translatedLines = useMemo(() => (chapterText ? chapterText.split("\n") : []), [chapterText]);
  const activeIndex = documents.findIndex((document) => document.path === activeDocument?.path);
  const review = useTranslationReview({ enabled: projectSettings.translationMode, projectRoot, activeName: activeDocument?.name, activeIndex, translatedLines, glossary: glossaryData });
  const isSplit = projectSettings.translationMode && review.hasOriginal && splitView;
  const wordCount = useMemo(() => {
    const text = textFromXhtml(editorText);
    return text ? text.split(/\s+/u).length : 0;
  }, [editorText]);

  // As opções de esforço dependem apenas do modelo documentado; a consulta é
  // local (sem rede) e não exige nova validação da chave. Ao trocar de modelo,
  // restauramos a escolha persistida ou aplicamos o padrão do modelo.
  useEffect(() => {
    const model = models[providerId];
    if (!model) { setEffortOptions(null); return; }
    let active = true;
    invoke<ModelEffortOptions>("get_model_effort_options", { provider: providerId, model })
      .then((options) => {
        if (!active) return;
        setEffortOptions(options);
        if (options.configurable) {
          setReasoningEfforts((current) => {
            const byProvider = { ...(current[providerId] ?? {}) };
            const existing = byProvider[model];
            if (existing && options.levels.some((level) => level.value === existing)) return current;
            return { ...current, [providerId]: { ...byProvider, [model]: options.default } };
          });
        }
      })
      .catch(() => { if (active) setEffortOptions(null); });
    return () => { active = false; };
  }, [models, providerId]);

  const selectedEffort = useMemo(() => {
    if (!effortOptions?.configurable) return "auto";
    const model = models[providerId];
    const persisted = model ? reasoningEfforts[providerId]?.[model] : undefined;
    if (persisted && effortOptions.levels.some((level) => level.value === persisted)) return persisted;
    return effortOptions.default;
  }, [effortOptions, models, providerId, reasoningEfforts]);

  const loadDocument = useCallback(async (document: ProjectDocument, page = 1) => {
    try {
      const text = await readTextFile(document.path);
      setActiveDocument(document);
      setEditorText(text);
      setSavedText(text);
      setAnalysis("");
      setFindings([]);
      setSuggestions([]);
      setAnalysisError("");
      setResumePage(page);
      setPagination({ page, pageCount: 1, xhtml: text });
    } catch (error) {
      await message(`Não foi possível ler ${document.name}.\n\n${String(error)}`, { title: "Falha ao abrir arquivo", kind: "error" });
    }
  }, []);

  const openProject = useCallback(async () => {
    setIsOpening(true);
    try {
      const selected = await open({ directory: true, multiple: false, recursive: true, title: "Escolha a pasta que contém os XHTML do livro" });
      if (!selected || Array.isArray(selected)) return;
      const files = await collectXhtmlFiles(selected);
      if (!files.length) {
        await message("Nenhum arquivo .xhtml ou .html foi encontrado nesta pasta. Escolha a pasta extraída do livro.", { title: "Arquivos XHTML não encontrados", kind: "info" });
        return;
      }
      setDocuments(files);
      const savedPosition = loadProjectPositions()[selected];
      const savedDocument = files.find((file) => file.path === savedPosition?.documentPath);
      setProjectRoot(selected);
      setProjectSettings(loadProjectSettings(selected));
      glossaryLoad(selected).then((state) => { setGlossary(glossaryToLines(state.glossary)); setGlossaryData(state.glossary); }).catch(() => { setGlossary([]); setGlossaryData(null); });
      await loadDocument(savedDocument ?? files[0], savedDocument ? savedPosition?.page : 1);
    } catch (error) {
      await message(String(error), { title: "Não foi possível abrir o projeto", kind: "error" });
    } finally {
      setIsOpening(false);
    }
  }, [loadDocument]);

  const analyzeDocument = useCallback(async () => {
    const pageText = scope === "chapter" ? paragraphTextFromXhtml(editorText) : paragraphTextFromXhtml(pagination.xhtml || editorText);
    if (!pageText.trim()) { setAnalysisError("Abra um capítulo XHTML antes de solicitar uma análise."); return; }
    if (!models[providerId].trim()) { setAnalysisError("Informe o modelo em Configurações antes de analisar."); setIsSettingsOpen(true); return; }
    if (!keyStatus[providerId]) { setAnalysisError("Salve e teste a chave deste provedor em Configurações antes de analisar."); setIsSettingsOpen(true); return; }
    setIsAnalyzing(true); setAnalysisError(""); setAnalysis(""); setFindings([]); setSuggestions([]); setGenesis(null); setMemoryNote("");
    try {
      let memory: string | null = null;
      if (profile?.usesMemory && projectSettings.useMemory && projectRoot) {
        const previous = documents.slice(0, Math.max(documents.findIndex((document) => document.path === activeDocument?.path), 0)).map((document) => document.name);
        const context = await vaultMemory(projectRoot, paragraphTextFromXhtml(editorText).split("\n"), previous).catch(() => null);
        memory = context?.text || null;
        if (context) setMemoryNote(context.text ? `Memória usada: ${context.recapsUsed.length} recap(s), ${context.notesUsed.length} nota(s).` : "O caderno não tem memória relevante para este capítulo.");
      }
      const result = await invoke<AnalysisResult>("analyze_open_document", { input: { provider: providerId, model: models[providerId], profile: profileId, text: pageText, reasoningEffort: effortOptions?.configurable ? selectedEffort : null, genre: projectSettings.genre, translationMode: projectSettings.translationMode, adultContent: projectSettings.adultContent, glossary, originalText: profile?.needsOriginal && review.hasOriginal ? (scope === "chapter" ? review.lines : originalSliceForPage(review.lines, pagination.page, pagination.pageCount)).join("\n") : null, memory } });
      setAnalysis(result.summary);
      setFindings(result.findings);
      setSuggestions(result.suggestions);
      setGenesis(result.genesis);
    } catch (error) { setAnalysisError(String(error)); } finally { setIsAnalyzing(false); }
  }, [activeDocument?.path, documents, editorText, effortOptions, glossary, keyStatus, models, pagination.page, pagination.pageCount, pagination.xhtml, profile?.needsOriginal, profile?.usesMemory, profileId, projectRoot, projectSettings, providerId, review.hasOriginal, review.lines, scope, selectedEffort]);

  const applySuggestion = useCallback((suggestion: EditorialSuggestion) => {
    const before = suggestion.before.trim();
    if (!before || !editorText.includes(before)) {
      setAnalysisError("Essa sugestão não foi aplicada porque o trecho original não foi localizado no XHTML atual. Revise-a manualmente.");
      return;
    }
    // Função de substituição: "$&" e similares em suggestion.after não podem virar padrões.
    setEditorText((current) => current.replace(before, () => suggestion.after));
    setExternalEditorRevision((current) => current + 1);
    setSuggestions((current) => current.filter((item) => item !== suggestion));
  }, [editorText]);

  const saveDocument = useCallback(async (): Promise<boolean> => {
    if (!activeDocument) {
      await message("Abra um capítulo XHTML antes de salvar.", { title: "Nenhum capítulo aberto", kind: "info" });
      return false;
    }
    if (editorText === savedText) return true;
    setIsSaving(true);
    try {
      await invoke<SaveResult>("save_xhtml_document", { path: activeDocument.path, content: editorText });
      setSavedText(editorText);
      return true;
    } catch (error) {
      await message(String(error), { title: "Não foi possível salvar", kind: "error" });
      return false;
    } finally {
      setIsSaving(false);
    }
  }, [activeDocument, editorText, savedText]);

  const requestDocument = useCallback((document: ProjectDocument) => {
    if (document.path === activeDocument?.path) return;
    if (activeDocument && editorText !== savedText) {
      setPendingDocument(document);
      return;
    }
    void loadDocument(document);
  }, [activeDocument, editorText, loadDocument, savedText]);

  const saveAndContinue = useCallback(async () => {
    if (!pendingDocument) return;
    const target = pendingDocument;
    const saved = await saveDocument();
    if (!saved) return;
    setPendingDocument(null);
    await loadDocument(target);
  }, [loadDocument, pendingDocument, saveDocument]);

  const discardAndContinue = useCallback(() => {
    if (!pendingDocument) return;
    const target = pendingDocument;
    setPendingDocument(null);
    void loadDocument(target);
  }, [loadDocument, pendingDocument]);

  const moveChapter = useCallback((direction: -1 | 1) => {
    const currentIndex = documents.findIndex((document) => document.path === activeDocument?.path);
    const next = documents[currentIndex + direction];
    if (next) requestDocument(next);
  }, [activeDocument?.path, documents, requestDocument]);

  const updateKeyStatus = useCallback((id: ProviderId, saved: boolean) => {
    setKeyStatus((current) => current[id] === saved ? current : { ...current, [id]: saved });
  }, []);

  const updatePagination = useCallback((next: PaginationState) => {
    setPagination((current) => current.page === next.page && current.pageCount === next.pageCount && current.xhtml === next.xhtml ? current : next);
    setResumePage(next.page);
    if (activeDocument) saveProjectPosition(projectRoot, activeDocument.path, next.page);
  }, [activeDocument, projectRoot]);

  return (
    <main className={`workspace-shell ${isAppDark ? "app-night" : ""}`}>
      <header className="topbar">
        <div className="brand"><img src="/editor-livros-epub.png" alt="" className="brand-icon" /><div><strong>Editor de Livros EPUB</strong><span>Local-first · XHTML · EPUB</span></div></div>
        <div className="topbar-actions"><button className="secondary-button" type="button" onClick={() => setIsNotebookOpen(true)} disabled={!projectRoot} title={projectRoot ? "Memória entre capítulos: personagens, lugares, fios e recaps" : "Abra um projeto"}>Caderno</button><button className="secondary-button" type="button" onClick={() => setIsExportOpen(true)} disabled={!projectRoot} title={projectRoot ? "Gerar o arquivo .epub final" : "Abra um projeto para exportar"}>Exportar EPUB</button><button className="secondary-button" type="button" onClick={() => setIsSettingsOpen(true)}>Configurações</button><button className="primary-button" type="button" onClick={openProject} disabled={isOpening}>{isOpening ? "Abrindo…" : "Abrir projeto"}</button></div>
      </header>
      <div className={isSplit ? "workspace-grid split-mode" : "workspace-grid"}>
        <aside className="library-panel" aria-label="Biblioteca do projeto">
          <div className="panel-heading"><span>PROJETO</span><button type="button" className="icon-button" onClick={openProject} aria-label="Abrir projeto XHTML">＋</button></div>
          {!documents.length ? <div className="empty-library"><div className="empty-mark">⌘</div><strong>Nenhum livro aberto</strong><p>Abra a pasta que contém os capítulos XHTML do livro.</p><button className="secondary-button" type="button" onClick={openProject} disabled={isOpening}>Escolher pasta XHTML</button></div> : <div className="project-browser"><strong title={projectRoot}>{fileNameFromPath(projectRoot)}</strong><span>{documents.length} capítulo{documents.length === 1 ? "" : "s"} encontrado{documents.length === 1 ? "" : "s"}</span><div className="chapter-list">{documents.map((document, index) => <button key={document.path} type="button" aria-current={document.path === activeDocument?.path ? "page" : undefined} className={document.path === activeDocument?.path ? "chapter-button active" : "chapter-button"} onClick={() => requestDocument(document)}><span className="chapter-index">{String(index + 1).padStart(2, "0")}</span><span className="chapter-name">{document.name}</span></button>)}</div></div>}
          <div className="safety-card"><span className="safety-dot" /><div><strong>Backup antes de salvar</strong><p>Confirmação e cópia do original antes de cada gravação.</p></div></div>
        </aside>
        <section className="editor-stage" aria-label="Editor e prévia">
          <div className="stage-toolbar"><div><span className="eyebrow">EDIÇÃO VISUAL</span><strong>{activeDocument?.name ?? "Abra um capítulo XHTML"}</strong></div><div className="reader-controls"><span>{wordCount.toLocaleString("pt-BR")} palavras</span><span className="reader-separator" /><button type="button" onClick={() => setFontScale((value) => Math.max(82, value - 8))} aria-label="Diminuir fonte">A−</button><button type="button" onClick={() => setFontScale((value) => Math.min(132, value + 8))} aria-label="Aumentar fonte">A+</button><label className="brightness-control" title="Brilho virtual da página"><span>☀</span><input type="range" min="55" max="115" value={brightness} onChange={(event) => setBrightness(Number(event.target.value))} aria-label="Brilho da página" /></label><button type="button" className="page-theme-button" onClick={() => setReaderTheme((theme) => theme === "warm" ? "paper" : theme === "paper" ? "night" : "warm")} aria-label="Alternar tema da página" title="Tema da página: sépia, claro ou noturno">◐</button><button type="button" className="theme-button" onClick={() => setIsAppDark((enabled) => !enabled)} aria-label="Alternar modo escuro do aplicativo" aria-pressed={isAppDark} title={isAppDark ? "Usar interface clara" : "Usar interface escura"}>{isAppDark ? "☀" : "☾"}</button><button className="save-button" type="button" onClick={saveDocument} disabled={!activeDocument || editorText === savedText || isSaving}>{isSaving ? "Salvando…" : "Salvar"}</button></div></div>
          <div className={isSplit ? "split-body" : "single-body"} style={isSplit ? undefined : { display: "flex", flex: 1, minHeight: 0, flexDirection: "column" }}>
            {isSplit && <OriginalPane chapters={review.chapters} index={review.index} isAuto={review.isAuto} onSelect={review.setIndex} sourceLabel={review.sourcePath.split(/[\\/]/u).filter(Boolean).pop() ?? "original"} page={pagination.page} pageCount={pagination.pageCount} report={review.report} />}
            <div className="split-editor" style={{ display: "flex", flex: 1, minHeight: 0, minWidth: 0, flexDirection: "column" }}>
          <VisualXhtmlEditor documentId={activeDocument?.path} documentName={activeDocument?.name} source={editorText} theme={readerTheme} fontScale={fontScale} brightness={brightness} externalRevision={externalEditorRevision} initialPage={resumePage} onChange={setEditorText} onPaginationChange={updatePagination} hasPreviousChapter={documents.findIndex((document) => document.path === activeDocument?.path) > 0} hasNextChapter={documents.findIndex((document) => document.path === activeDocument?.path) < documents.length - 1} onPreviousChapter={() => moveChapter(-1)} onNextChapter={() => moveChapter(1)} onSaveRequested={saveDocument} />
            </div>
          </div>
          <div className="reader-footer"><span>{activeDocument ? editorText === savedText ? "Todas as alterações estão salvas" : "Alterações não gravadas — o XHTML será atualizado quando você salvar" : "Nenhum arquivo carregado"}</span><span>Página {pagination.page} de {pagination.pageCount} · Ctrl + S para salvar</span></div>
        </section>
        <aside className="ai-panel" aria-label="Assistente editorial">
          <div className="panel-heading"><span>ASSISTENTE EDITORIAL</span><span className="live-dot">●</span></div>
          <div className="ai-intro"><span className="ai-orb">✦</span><div><strong>Skill editorial estruturada</strong><p>A IA sugere; você revisa e aplica com diff.</p></div></div>
          <div className="provider-status"><span>{provider.badge}</span><div><strong>{provider.label}</strong><small>{models[providerId] ? friendlyModelName(models[providerId]) : "Conecte uma chave para carregar os modelos"}</small><small>{keyStatus[providerId] ? availableModels[providerId].length ? `✓ ${availableModels[providerId].length} modelos neste servidor` : "Chave salva; catálogo ainda não carregado" : "Chave não configurada"}</small></div></div>
          <label className="field-label quick-model-field">MODELO DE ANÁLISE<input className="model-search" type="search" value={modelQuery} onChange={(event) => setModelQuery(event.target.value)} placeholder="Buscar por modelo ou servidor…" disabled={!hasAnyCatalog} aria-label="Buscar modelo por nome, identificador ou servidor" /><select value={selectedModelKey} onChange={(event) => chooseModel(event.target.value)} disabled={!hasAnyCatalog} aria-label="Modelo usado na próxima análise"><option value="">{hasAnyCatalog ? modelGroups.length ? "Selecione um modelo" : "Nenhum modelo encontrado" : "Conecte uma chave para carregar os modelos"}</option>{modelGroups.map((group) => <optgroup key={group.provider.id} label={`${group.provider.label} · ${group.models.length}`}>{group.models.map((availableModel) => <option key={`${group.provider.id}::${availableModel}`} value={`${group.provider.id}::${availableModel}`}>{friendlyModelName(availableModel)} · {availableModel}</option>)}</optgroup>)}</select><small>Cada grupo é um servidor; o modelo escolhido é enviado a ele.</small></label>
          <label className="field-label effort-field">ESFORÇO DE RACIOCÍNIO
            <select value={selectedEffort} onChange={(event) => { const model = models[providerId]; if (!model) return; setReasoningEfforts((current) => ({ ...current, [providerId]: { ...(current[providerId] ?? {}), [model]: event.target.value } })); }} disabled={!hasAnyCatalog || !effortOptions?.configurable} aria-label="Esforço de raciocínio usado na próxima análise">
              {effortOptions?.configurable ? effortOptions.levels.map((level) => <option key={level.value} value={level.value}>{level.label}</option>) : <option value="auto">Automático</option>}
            </select>
            <small>{effortOptions?.note || (models[providerId] ? "" : "Selecione um modelo para ver as opções de esforço.")}</small>
          </label>
          <button className="secondary-button settings-link" type="button" onClick={() => setIsSettingsOpen(true)}>Configurar provedor e chave</button>
          <label className="field-label">PERFIL EDITORIAL<select value={profile?.id ?? profileId} onChange={(event) => changeProfile(event.target.value)} disabled={!catalog}>{catalog ? catalog.profiles.map((item) => <option key={item.id} value={item.id} disabled={item.needsOriginal && !review.hasOriginal}>{item.label}{item.needsOriginal && !review.hasOriginal ? " (carregue o original)" : ""}</option>) : <option>Carregando…</option>}</select></label>
          {profile && <div className="profile-card"><span>{scope === "chapter" ? "Capítulo inteiro" : "Página aberta"}</span><strong>{profile.shortLabel}</strong><p>{profile.description}</p><small>{profile.output}</small><small className="tier-hint">Nível de modelo sugerido pela skill: {profile.tier}{profile.tier === "Rápido" ? " (modelos menores respondem mais rápido)" : ""}</small>{profile.usesMemory && <small className="tier-hint">Usa a memória do caderno (recaps e notas) quando houver.</small>}</div>}
          <label className="field-label">ESCOPO DA ANÁLISE<select value={scope} onChange={(event) => setScope(event.target.value as "page" | "chapter")}><option value="page">Página aberta (mais rápido)</option><option value="chapter">Capítulo inteiro (até 60 mil caracteres)</option></select></label>
          <div className="project-settings"><strong>Projeto</strong>
            <label className="field-label">GÊNERO (calibra metas e limiares)<select value={projectSettings.genre} onChange={(event) => setProjectSettings((current) => ({ ...current, genre: event.target.value as GenreId }))}>{catalog?.genres.map((item) => <option key={item.id} value={item.id}>{item.label}</option>)}</select></label>
            <label className="check"><input type="checkbox" checked={projectSettings.translationMode} onChange={(event) => setProjectSettings((current) => ({ ...current, translationMode: event.target.checked }))} /><span>É tradução EN→PT-BR (alerta suavizações e sugere fluidez)</span></label>
            {projectSettings.translationMode && <div className="original-source">
              {review.hasOriginal ? <><small>Original: {review.sourcePath} ({review.chapters.length} capítulos)</small>
                <label className="check"><input type="checkbox" checked={splitView} onChange={(event) => setSplitView(event.target.checked)} /><span>Tela dividida (original à esquerda)</span></label>
                <button className="secondary-button" type="button" onClick={review.clear}>Remover original</button></>
                : <><button className="secondary-button" type="button" onClick={() => void review.choose("epub")} disabled={review.loading}>{review.loading ? "Carregando…" : "Carregar original (EPUB)…"}</button>
                  <button className="secondary-button" type="button" onClick={() => void review.choose("folder")} disabled={review.loading}>Carregar original (pasta)…</button></>}
              {review.error && <small role="alert" style={{ color: "#a3404d" }}>{review.error}</small>}
            </div>}
            <label className="check"><input type="checkbox" checked={projectSettings.useMemory} onChange={(event) => setProjectSettings((current) => ({ ...current, useMemory: event.target.checked }))} /><span>Usar a memória do caderno nas análises que a pedem</span></label>
            <label className="check"><input type="checkbox" checked={projectSettings.adultContent} onChange={(event) => setProjectSettings((current) => ({ ...current, adultContent: event.target.checked }))} /><span>Contém cenas adultas (preservar intensidade, sem suavizar)</span></label>
            {glossary.length > 0 && <p className="glossary-chip">✓ glossário com {glossary.length} entradas enviado à IA</p>}
            <button className="secondary-button" type="button" disabled={!projectRoot} onClick={() => setIsGlossaryOpen(true)}>{glossary.length > 0 ? "Editar glossário…" : "Criar glossário…"}</button>
          </div>
          <button className="primary-button wide-button" type="button" onClick={analyzeDocument} disabled={isAnalyzing || !activeDocument}>{isAnalyzing ? (scope === "chapter" ? "Analisando capítulo…" : `Analisando página ${pagination.page}…`) : (scope === "chapter" ? "Analisar capítulo" : `Analisar página ${pagination.page}`)}</button>{isAnalyzing && <div className="analysis-progress" role="status" aria-live="polite"><span>{progressLabel(aiProgress)}</span><button type="button" className="discard-button" onClick={() => void invoke("cancel_ai_analysis")}>Cancelar</button></div>}<p className="privacy-note">Apenas o trecho analisado (página ou capítulo, conforme o escopo) é enviado ao provedor escolhido. A chave nunca entra no XHTML ou EPUB.</p>
          {memoryNote && <p className="privacy-note">{memoryNote}</p>}{genesis && <GenesisCard genesis={genesis} genreLabel={genreLabel} />}{(analysisError || analysis || findings.length > 0 || suggestions.length > 0) && <section className={analysisError ? "analysis-result error" : "analysis-result"} aria-live="polite"><strong>{analysisError ? "Não foi possível analisar" : "Resultado da análise"}</strong>{analysisError ? <p>{analysisError}</p> : <>{analysis && <p className="analysis-summary">{analysis}</p>}{findings.length > 0 && <div className="finding-list"><span className="finding-heading">Achados editoriais</span>{findings.map((finding, index) => <article className="finding-card" key={`${finding.title}-${index}`}><header><strong>{finding.title}</strong><span className={`severity ${finding.severity.toLocaleLowerCase("pt-BR")}`}>{finding.severity}</span></header><p>{finding.detail}</p>{finding.anchor && <blockquote><strong>Trecho:</strong> “{finding.anchor}”</blockquote>}</article>)}</div>}{suggestions.length > 0 && <div className="suggestion-list"><span className="suggestion-heading">Sugestões aplicáveis</span>{suggestions.map((suggestion, index) => <article className="suggestion-card" key={`${suggestion.before}-${index}`}><div><strong>Antes — texto atual</strong><del>{suggestion.before}</del></div><div><strong>Depois — proposta</strong><ins>{suggestion.after}</ins></div><small>{suggestion.reason}</small><button type="button" onClick={() => applySuggestion(suggestion)}>Aplicar sugestão</button></article>)}</div>}</>}</section>}
          {projectSettings.translationMode && review.report && <FidelityCard report={review.report} />}
          <ScanPanel text={chapterText} options={scanOptions} genreLabel={genreLabel} onApplyFluency={(fix) => applySuggestion({ before: fix.before, after: fix.after, reason: fix.reason })} />
          <div className="module-row"><span>Módulos do núcleo</span><strong>{bootstrap ? `${bootstrap.modules.length} preparados` : "carregando"}</strong></div>
        </aside>
      </div>
      {isSettingsOpen && <AiSettingsDialog activeProviderId={providerId} models={models} availableModels={availableModels} keyStatus={keyStatus} onClose={() => setIsSettingsOpen(false)} onProviderChange={setProviderId} onModelChange={(id, model) => setModels((current) => ({ ...current, [id]: model }))} onModelsLoaded={(id, loadedModels) => setAvailableModels((current) => ({ ...current, [id]: loadedModels }))} onKeyStatusChange={updateKeyStatus} />}
      {isNotebookOpen && <NotebookDialog projectRoot={projectRoot} chapters={documents} activeChapterIndex={Math.max(activeIndex, 0)} activeChapterLines={translatedLines} ai={{ providerId, model: models[providerId], reasoningEffort: effortOptions?.configurable ? selectedEffort : null, ready: Boolean(keyStatus[providerId] && models[providerId]) }} genre={projectSettings.genre} glossaryLines={glossary} glossaryData={glossaryData} onClose={() => setIsNotebookOpen(false)} onChanged={() => setMemoryNote("")} />}
      {isGlossaryOpen && <GlossaryDialog projectRoot={projectRoot} chapterPaths={documents.map((document) => document.path)} onClose={() => setIsGlossaryOpen(false)} onSaved={(saved: Glossary) => { setGlossary(glossaryToLines(saved)); setGlossaryData(saved); }} />}
      {isExportOpen && <ExportEpubDialog projectRoot={projectRoot} chapterPaths={documents.map((document) => document.path)} hasUnsavedChanges={Boolean(activeDocument) && editorText !== savedText} onClose={() => setIsExportOpen(false)} />}
      {pendingDocument && <div className="transition-modal-backdrop" role="presentation"><section className="transition-modal" role="dialog" aria-modal="true" aria-labelledby="save-changes-title"><span className="transition-icon">✦</span><div><span className="eyebrow">ALTERAÇÕES PENDENTES</span><h2 id="save-changes-title">Salvar edições e continuar?</h2><p>Você editou <strong>{activeDocument?.name}</strong>. O próximo capítulo será aberto somente depois de salvar ou descartar estas alterações.</p></div><div className="transition-actions"><button type="button" className="secondary-button" onClick={() => setPendingDocument(null)} disabled={isSaving}>Cancelar</button><button type="button" className="discard-button" onClick={discardAndContinue} disabled={isSaving}>Descartar e continuar</button><button type="button" className="primary-button" onClick={() => void saveAndContinue()} disabled={isSaving}>{isSaving ? "Salvando…" : "Salvar edições e continuar"}</button></div><small>Um backup do XHTML atual é criado automaticamente.</small></section></div>}
    </main>
  );
}
