import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { join } from "@tauri-apps/api/path";
import { message, open } from "@tauri-apps/plugin-dialog";
import { readDir, readTextFile } from "@tauri-apps/plugin-fs";
import { useCallback, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { emptyRecord, friendlyModelName, providerFor, providerIds, providers, type ProviderId } from "../ai/providers";
import { applyTextSuggestion } from "../editor/applySuggestion";
import { paragraphTextFromXhtml, textFromXhtml } from "../editor/xhtmlDocument";
import { VisualXhtmlEditor, type PaginationState, type ReaderTheme } from "../editor/VisualXhtmlEditor";
import type { EditorialProfileId } from "../editorial/catalog";
import { isProjectBroken, loadProject, patchProject, toStoredPath } from "../project/projectStore";
import { useDialogA11y } from "../../shared/useDialogA11y";
import { BibleDialog } from "../notebook/BibleDialog";
import { allowRecentProject, recentProjects, resumeLabel, touchRecentProject, type RecentProject } from "../recent/recentApi";
import { AssistantPanel } from "../assistant/AssistantPanel";
import { ResultsPanel, type ResultsTab } from "../assistant/ResultsPanel";
import { ReferencesPanel } from "../references/ReferencesPanel";
import { ReplaceDialog } from "../replace/ReplaceDialog";
import { samePath } from "../replace/replaceApi";
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
const defaultProjectSettings: ProjectSettings = { genre: "comercial", translationMode: false, adultContent: false, useMemory: true };

function settingsFromFile(file: Awaited<ReturnType<typeof loadProject>>["file"]): ProjectSettings {
  const c = file.configuracoes ?? {};
  return { genre: (c.genero as GenreId) ?? defaultProjectSettings.genre, translationMode: c.modoTraducao ?? false, adultContent: c.cenasAdultas ?? false, useMemory: c.usarMemoria ?? true };
}

const settingsToFile = (s: ProjectSettings) => ({ genero: s.genre, modoTraducao: s.translationMode, cenasAdultas: s.adultContent, usarMemoria: s.useMemory });

interface AiProgress { elapsedMs: number; answerChars: number; reasoningChars: number; phase: "waiting" | "thinking" | "writing"; }
interface SaveResult { backupPath: string; }
interface EffortOption { value: string; label: string; }
interface ModelEffortOptions { configurable: boolean; levels: EffortOption[]; default: string; note: string; }

const initialModels = (): Record<ProviderId, string> => emptyRecord(() => "");
const initialAvailableModels = (): Record<ProviderId, string[]> => emptyRecord<string[]>(() => []);
const initialReasoningEfforts = (): Record<ProviderId, Record<string, string>> => emptyRecord<Record<string, string>>(() => ({}));
const AI_PREFERENCES_KEY = "book-editor.ai-preferences.v1";

interface PersistedAiPreferences {
  providerId?: ProviderId;
  models?: Record<ProviderId, string>;
  availableModels?: Record<ProviderId, string[]>;
  validatedProviders?: Partial<Record<ProviderId, boolean>>;
  reasoningEfforts?: Record<ProviderId, Record<string, string>>;
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

/** Pergunta "salvar antes de trocar de capítulo?": Esc cancela, Tab fica preso no diálogo. */
function PendingDocumentModal({ onCancel, busy, children }: { onCancel: () => void; busy: boolean; children: ReactNode }) {
  const ref = useDialogA11y<HTMLElement>(() => { if (!busy) onCancel(); });
  return <div className="transition-modal-backdrop" role="presentation"><section ref={ref} tabIndex={-1} className="transition-modal" role="dialog" aria-modal="true" aria-labelledby="save-changes-title">{children}</section></div>;
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
  const [isReplaceOpen, setIsReplaceOpen] = useState(false);
  const [isReferencesOpen, setIsReferencesOpen] = useState(false);
  const [isBibleOpen, setIsBibleOpen] = useState(false);
  const [resultsOpen, setResultsOpen] = useState(false);
  const [resultsTab, setResultsTab] = useState<ResultsTab>("resultado");
  const [recents, setRecents] = useState<RecentProject[]>([]);
  const [notebookStart, setNotebookStart] = useState("");
  const [replaceStart, setReplaceStart] = useState("");
  const [memoryNote, setMemoryNote] = useState("");
  const [glossaryData, setGlossaryData] = useState<Glossary | null>(null);
  const [splitView, setSplitView] = useState(true);
  const [genesis, setGenesis] = useState<Genesis | null>(null);
  const [, setBootstrap] = useState<AppBootstrap | null>(null);
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
  const [suggestionNotice, setSuggestionNotice] = useState("");
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
  const positionTimer = useRef<number | undefined>(undefined);
  const pendingPosition = useRef<{ root: string; value: { documento: string; pagina: number } } | null>(null);
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
  // Configurações do projeto vivem em .livro/projeto.json. Só gravamos depois de carregar o arquivo
  // do projeto aberto e quando algo realmente mudou (evita sobrescrever com os padrões ao abrir).
  const settingsReady = useRef("");
  const lastSettings = useRef("");
  useEffect(() => {
    if (!projectRoot || settingsReady.current !== projectRoot) return;
    const serialized = JSON.stringify(settingsToFile(projectSettings));
    if (serialized === lastSettings.current) return;
    lastSettings.current = serialized;
    void patchProject(projectRoot, "configuracoes", settingsToFile(projectSettings)).catch(() => undefined);
  }, [projectRoot, projectSettings]);
  const changeProfile = useCallback((id: EditorialProfileId) => {
    setProfileId(id);
    const next = catalog?.profiles.find((item) => item.id === id);
    if (next) setScope(next.suggestedScope);
  }, [catalog]);
  const chapterText = useMemo(() => (editorText ? paragraphTextFromXhtml(editorText) : ""), [editorText]);
  const scanOptions = useMemo<ScanOptions>(() => ({ genre: projectSettings.genre, translationMode: projectSettings.translationMode, adultContent: projectSettings.adultContent, isOpening: true }), [projectSettings]);
  const pageReferenceLines = useMemo(() => (isReferencesOpen ? paragraphTextFromXhtml(pagination.xhtml || editorText).split("\n") : []), [editorText, isReferencesOpen, pagination.xhtml]);
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
      setSuggestionNotice("");
      setResumePage(page);
      setPagination({ page, pageCount: 1, xhtml: text });
    } catch (error) {
      await message(`Não foi possível ler ${document.name}.\n\n${String(error)}`, { title: "Falha ao abrir arquivo", kind: "error" });
    }
  }, []);

  /** Abre uma pasta de livro já escolhida: restaura configurações, capítulo e página onde a leitura parou. */
  const openProjectFolder = useCallback(async (selected: string) => {
      const files = await collectXhtmlFiles(selected);
      if (!files.length) {
        await message("Nenhum arquivo .xhtml ou .html foi encontrado nesta pasta. Escolha a pasta extraída do livro.", { title: "Arquivos XHTML não encontrados", kind: "info" });
        return;
      }
      setDocuments(files);
      const project = await loadProject(selected, { fresh: true });
      if (project.error) await message(`${project.error}\n\nO projeto abre com os padrões e as configurações não serão gravadas até o arquivo ser corrigido.`, { title: "Arquivo do projeto ilegível", kind: "warning" });
      const stored = project.file.leitura;
      const savedDocument = stored ? files.find((file) => toStoredPath(selected, file.path).toLowerCase() === stored.documento.toLowerCase()) : undefined;
      const loadedSettings = settingsFromFile(project.file);
      settingsReady.current = ""; lastSettings.current = JSON.stringify(settingsToFile(loadedSettings));
      setProjectRoot(selected);
      setProjectSettings(loadedSettings);
      settingsReady.current = selected;
      glossaryLoad(selected).then((state) => { setGlossary(glossaryToLines(state.glossary)); setGlossaryData(state.glossary); }).catch(() => { setGlossary([]); setGlossaryData(null); });
      await loadDocument(savedDocument ?? files[0], savedDocument ? stored?.pagina : 1);
    void touchRecentProject(selected).then((list) => setRecents(Array.isArray(list) ? list : [])).catch(() => undefined);
  }, [loadDocument]);

  // Alterações do capítulo aberto não se perdem ao trocar de livro.
  // Grava já a posição de leitura pendente (ao trocar de livro ou fechar a janela).
  const flushPositionRef = useRef<() => Promise<void>>(async () => undefined);
  flushPositionRef.current = async () => {
    if (positionTimer.current) window.clearTimeout(positionTimer.current);
    const pending = pendingPosition.current;
    pendingPosition.current = null;
    if (pending) await patchProject(pending.root, "leitura", pending.value).catch(() => undefined);
  };

  const openProject = useCallback(async () => {
    setIsOpening(true);
    try {
      if (unsavedRef.current && !(await saveRef.current())) return;
      const selected = await open({ directory: true, multiple: false, recursive: true, title: "Escolha a pasta que contém os XHTML do livro" });
      if (!selected || Array.isArray(selected)) return;
      await flushPositionRef.current();
      await openProjectFolder(selected);
    } catch (error) {
      await message(String(error), { title: "Não foi possível abrir o projeto", kind: "error" });
    } finally {
      setIsOpening(false);
    }
  }, [openProjectFolder]);

  const lastWork = recents.find((item) => item.exists) ?? null;
  const normalizePath = (path: string) => path.replace(/[\\/]+$/u, "").replace(/\\/gu, "/").toLowerCase();
  const alreadyOpen = lastWork !== null && projectRoot !== "" && normalizePath(lastWork.path) === normalizePath(projectRoot);

  /** "Abrir último trabalho": o livro mais recente, já no capítulo e na página onde parou. */
  const openLastWork = useCallback(async () => {
    const target = recents.find((item) => item.exists);
    if (!target) {
      await message(recents.length ? `A pasta do último trabalho não foi encontrada:\n${recents[0].path}\n\nEla pode ter sido movida ou estar em uma unidade desconectada. Use “Abrir projeto” para localizá-la.` : "Ainda não há um trabalho anterior. Use “Abrir projeto” para escolher a pasta do livro.", { title: "Último trabalho indisponível", kind: "info" });
      return;
    }
    setIsOpening(true);
    try {
      if (unsavedRef.current && !(await saveRef.current())) return;
      await flushPositionRef.current();
      await allowRecentProject(target.path);
      await openProjectFolder(target.path);
    } catch (error) {
      await message(String(error), { title: "Não foi possível abrir o último trabalho", kind: "error" });
    } finally {
      setIsOpening(false);
    }
  }, [openProjectFolder, recents]);

  useEffect(() => { recentProjects().then((list) => setRecents(Array.isArray(list) ? list : [])).catch(() => setRecents([])); }, []);

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
    const next = applyTextSuggestion(editorText, suggestion.before, suggestion.after);
    if (next === null) {
      setSuggestionNotice("Não apliquei esta sugestão: o trecho “antes” não bate com o texto atual (talvez já tenha sido editado). Localize-o no capítulo e ajuste à mão.");
      return;
    }
    setSuggestionNotice("");
    setEditorText(next);
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
      await invoke<SaveResult>("save_xhtml_document", { root: projectRoot, path: activeDocument.path, content: editorText });
      setSavedText(editorText);
      return true;
    } catch (error) {
      await message(String(error), { title: "Não foi possível salvar", kind: "error" });
      return false;
    } finally {
      setIsSaving(false);
    }
  }, [activeDocument, editorText, projectRoot, savedText]);

  // Quando uma análise termina, o painel de resultados abre sozinho; ao trocar de capítulo (resultados limpos) ele fecha.
  useEffect(() => {
    if (analysisError || analysis || findings.length > 0 || suggestions.length > 0) { setResultsTab(analysisError || suggestions.length === 0 ? "resultado" : "sugestoes"); setResultsOpen(true); }
    else setResultsOpen((open) => (resultsTab === "diagnostico" ? open : false));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [analysis, analysisError, findings]);

  // Atalhos: Alt+R referências, Alt+H substituir no livro, Alt+N caderno (Alt+setas já viram página).
  const shortcutRef = useRef<{ references: () => void; replace: () => void; notebook: () => void }>({ references: () => undefined, replace: () => undefined, notebook: () => undefined });
  shortcutRef.current = {
    references: () => { if (projectRoot && activeDocument) setIsReferencesOpen((open) => !open); },
    replace: () => { if (projectRoot && documents.length > 0) void openReplace(); },
    notebook: () => { if (projectRoot) { setNotebookStart(""); setIsNotebookOpen(true); } },
  };
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (!event.altKey || event.ctrlKey || event.metaKey || event.shiftKey) return;
      const key = event.key.toLowerCase();
      const action = key === "r" ? "references" : key === "h" ? "replace" : key === "n" ? "notebook" : null;
      if (!action || (action !== "references" && document.querySelector("[role=dialog][aria-modal=true]"))) return;
      event.preventDefault();
      shortcutRef.current[action]();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  // Fechar a janela com alterações não salvas: pergunta em vez de perder o trabalho.
  const unsavedRef = useRef(false);
  unsavedRef.current = Boolean(activeDocument) && editorText !== savedText;
  const saveRef = useRef(saveDocument);
  saveRef.current = saveDocument;
  useEffect(() => {
    let active = true;
    let stop: (() => void) | undefined;
    try {
      void getCurrentWindow().onCloseRequested(async (event) => {
        if (!unsavedRef.current) {
          // Sem edições pendentes, mas a última página virada ainda pode não ter sido gravada.
          if (!pendingPosition.current) return;
          event.preventDefault();
          await flushPositionRef.current();
          await getCurrentWindow().destroy();
          return;
        }
        event.preventDefault();
        const choice = await message("Há alterações não salvas neste capítulo.", { title: "Salvar antes de sair?", kind: "warning", buttons: { yes: "Salvar e sair", no: "Sair sem salvar", cancel: "Continuar editando" } });
        if (choice === "Salvar e sair" && !(await saveRef.current())) return;
        if (choice === "Salvar e sair" || choice === "Sair sem salvar") { await flushPositionRef.current(); await getCurrentWindow().destroy(); }
      }).then((fn) => { if (active) stop = fn; else fn(); }).catch(() => undefined);
    } catch { /* fora do Tauri (testes no navegador) */ }
    return () => { active = false; stop?.(); };
  }, []);

  // Substituir no livro grava em disco: o capítulo aberto é salvo antes e recarregado depois.
  const openReplace = useCallback(async () => {
    if (activeDocument && editorText !== savedText && !(await saveDocument())) return;
    setIsReplaceOpen(true);
  }, [activeDocument, editorText, saveDocument, savedText]);

  const onChapterFilesChanged = useCallback((changed: string[]) => {
    if (activeDocument && changed.some((path) => samePath(path, activeDocument.path))) void loadDocument(activeDocument, pagination.page);
  }, [activeDocument, loadDocument, pagination.page]);

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
    if (activeDocument && projectRoot && !isProjectBroken(projectRoot)) {
      // A posição muda a cada página virada: gravamos só depois de 700 ms de calma.
      const target = { documento: toStoredPath(projectRoot, activeDocument.path), pagina: next.page };
      if (positionTimer.current) window.clearTimeout(positionTimer.current);
      pendingPosition.current = { root: projectRoot, value: target };
      positionTimer.current = window.setTimeout(() => { const pending = pendingPosition.current; pendingPosition.current = null; if (pending) void patchProject(pending.root, "leitura", pending.value).catch(() => undefined); }, 700);
    }
  }, [activeDocument, projectRoot]);

  return (
    <main className={`workspace-shell ${isAppDark ? "app-night" : ""}`}>
      <header className="topbar">
        <div className="brand"><img src="/editor-livros-epub.png" alt="" className="brand-icon" /><div><h1 className="brand-title">Editor de Livros EPUB</h1><span>Local-first · XHTML · EPUB</span></div></div>
        {activeDocument && <div className="crumbs" aria-label="Local atual"><strong title={projectRoot}>{fileNameFromPath(projectRoot)}</strong><span aria-hidden="true">›</span><span>{activeDocument.name}</span><span>· página {pagination.page} de {pagination.pageCount}</span></div>}
        <div className="topbar-actions">
          <button className="secondary-button" type="button" onClick={() => void openLastWork()} disabled={isOpening || recents.length === 0 || alreadyOpen} title={alreadyOpen ? "Este já é o livro aberto" : lastWork ? `${lastWork.name}${resumeLabel(lastWork) ? ` — ${resumeLabel(lastWork)}` : ""}` : recents.length ? `Pasta não encontrada: ${recents[0].path}` : "Ainda não há um trabalho anterior"}>Abrir último trabalho</button>
          <button className={projectRoot ? "secondary-button" : "primary-button"} type="button" onClick={openProject} disabled={isOpening}>{isOpening ? "Abrindo…" : "Abrir projeto"}</button>
          <button className={projectRoot ? "primary-button" : "secondary-button"} type="button" onClick={() => setIsExportOpen(true)} disabled={!projectRoot} title={projectRoot ? "Gerar o arquivo .epub final" : "Abra um projeto para exportar"}>Exportar EPUB</button>
          <button className="secondary-button" type="button" onClick={() => setIsSettingsOpen(true)}>Configurações</button>
        </div>
      </header>
      <div className={isSplit ? "workspace-grid split-mode" : "workspace-grid"}>
        <aside className="library-panel" aria-label="Biblioteca do projeto">
          <div className="panel-heading"><span>PROJETO</span><button type="button" className="icon-button" onClick={openProject} aria-label="Abrir projeto XHTML">＋</button></div>
          {!documents.length ? <div className="empty-library"><div className="empty-mark">⌘</div><strong>Nenhum livro aberto</strong><p>Abra a pasta que contém os capítulos XHTML do livro.</p>{lastWork && <button className="primary-button resume-button" type="button" onClick={() => void openLastWork()} disabled={isOpening}><span>Continuar: {lastWork.name}</span>{resumeLabel(lastWork) && <small>{resumeLabel(lastWork)}</small>}</button>}<button className="secondary-button" type="button" onClick={openProject} disabled={isOpening}>Escolher pasta XHTML</button></div> : <div className="project-browser"><strong title={projectRoot}>{fileNameFromPath(projectRoot)}</strong><span>{documents.length} capítulo{documents.length === 1 ? "" : "s"} encontrado{documents.length === 1 ? "" : "s"}</span><div className="chapter-list">{documents.map((document, index) => <button key={document.path} type="button" aria-current={document.path === activeDocument?.path ? "page" : undefined} className={document.path === activeDocument?.path ? "chapter-button active" : "chapter-button"} onClick={() => requestDocument(document)}><span className="chapter-index">{String(index + 1).padStart(2, "0")}</span><span className="chapter-name">{document.name}</span></button>)}</div></div>}
          <nav className="book-tools" aria-label="Ferramentas do livro">
            <div className="tools-group" role="group" aria-labelledby="tools-consult">
              <span className="tools-heading" id="tools-consult">CONSULTAR</span>
              <button type="button" onClick={() => setIsReferencesOpen((open) => !open)} disabled={!projectRoot || !activeDocument} aria-pressed={isReferencesOpen} title="Para os nomes citados nesta página: ficha, ligações, registros e bíblia do caderno"><span>Referências</span><kbd>Alt+R</kbd></button>
              <button type="button" onClick={() => { setNotebookStart(""); setIsNotebookOpen(true); }} disabled={!projectRoot} title="Memória entre capítulos: personagens, lugares, fios, registros e recaps"><span>Caderno</span><kbd>Alt+N</kbd></button>
              <button type="button" onClick={() => setIsBibleOpen(true)} disabled={!projectRoot} title="Fatos canônicos da série, do livro ou do arco, por seção, com IA opcional"><span>Bíblia</span></button>
            </div>
            <div className="tools-group" role="group" aria-labelledby="tools-edit">
              <span className="tools-heading" id="tools-edit">EDITAR</span>
              <button type="button" onClick={() => void openReplace()} disabled={!projectRoot || documents.length === 0} title="Troca palavras inteiras (ex.: nome de personagem) com prévia e desfazer"><span>Substituir no livro</span><kbd>Alt+H</kbd></button>
              <button type="button" onClick={() => setIsGlossaryOpen(true)} disabled={!projectRoot}><span>{glossary.length > 0 ? "Glossário" : "Criar glossário"}</span></button>
            </div>
          </nav>
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
        <AssistantPanel
          model={{ badge: provider.badge, provider: provider.label, name: models[providerId] ? friendlyModelName(models[providerId]) : null, effort: effortOptions?.configurable ? effortOptions.levels.find((level) => level.value === selectedEffort)?.label ?? null : null, ready: Boolean(keyStatus[providerId] && models[providerId]) }}
          modelPicker={<>
            <div className="provider-status"><span>{provider.badge}</span><div><strong>{provider.label}</strong><small>{keyStatus[providerId] ? availableModels[providerId].length ? `✓ ${availableModels[providerId].length} modelos neste servidor` : "Chave salva; catálogo ainda não carregado" : "Chave não configurada"}</small></div></div>
            <label className="field-label quick-model-field">MODELO<input className="model-search" type="search" value={modelQuery} onChange={(event) => setModelQuery(event.target.value)} placeholder="Buscar por modelo ou servidor" disabled={!hasAnyCatalog} aria-label="Buscar modelo por nome, identificador ou servidor" /><select value={selectedModelKey} onChange={(event) => chooseModel(event.target.value)} disabled={!hasAnyCatalog} aria-label="Modelo usado na próxima análise"><option value="">{hasAnyCatalog ? modelGroups.length ? "Selecione um modelo" : "Nenhum modelo encontrado" : "Conecte uma chave para carregar os modelos"}</option>{modelGroups.map((group) => <optgroup key={group.provider.id} label={`${group.provider.label} · ${group.models.length}`}>{group.models.map((availableModel) => <option key={`${group.provider.id}::${availableModel}`} value={`${group.provider.id}::${availableModel}`}>{friendlyModelName(availableModel)} · {availableModel}</option>)}</optgroup>)}</select><small>Cada grupo é um servidor; o modelo escolhido é enviado a ele.</small></label>
            <label className="field-label effort-field">ESFORÇO DE RACIOCÍNIO
              <select value={selectedEffort} onChange={(event) => { const model = models[providerId]; if (!model) return; setReasoningEfforts((current) => ({ ...current, [providerId]: { ...(current[providerId] ?? {}), [model]: event.target.value } })); }} disabled={!hasAnyCatalog || !effortOptions?.configurable} aria-label="Esforço de raciocínio usado na próxima análise">
                {effortOptions?.configurable ? effortOptions.levels.map((level) => <option key={level.value} value={level.value}>{level.label}</option>) : <option value="auto">Automático</option>}
              </select>
              <small>{effortOptions?.note || (models[providerId] ? "" : "Selecione um modelo para ver as opções de esforço.")}</small>
            </label>
            <button className="secondary-button" type="button" onClick={() => setIsSettingsOpen(true)}>Configurar provedor e chave</button>
          </>}
          profiles={catalog ? catalog.profiles.map((item) => ({ id: item.id, label: item.label, description: item.description, tier: item.tier, usesMemory: item.usesMemory, disabled: item.needsOriginal && !review.hasOriginal })) : []}
          profileId={profile?.id ?? profileId}
          onProfile={(id) => changeProfile(id as EditorialProfileId)}
          scope={scope}
          onScope={setScope}
          pageLabel={`Página ${pagination.page}`}
          onAnalyze={analyzeDocument}
          analyzing={isAnalyzing}
          progressText={progressLabel(aiProgress)}
          onCancel={() => void invoke("cancel_ai_analysis")}
          blocker={!activeDocument ? "Abra um capítulo para analisar." : !keyStatus[providerId] ? "Conecte a chave de um provedor (botão do modelo, acima)." : !models[providerId] ? "Escolha um modelo no botão acima." : null}
          results={{ findings: findings.length, suggestions: suggestions.length, error: Boolean(analysisError), hasAny: Boolean(analysisError || analysis || findings.length || suggestions.length) }}
          onOpenResults={() => { setResultsTab(suggestions.length ? "sugestoes" : "resultado"); setResultsOpen(true); }}
          onOpenDiagnostics={() => { setResultsTab("diagnostico"); setResultsOpen(true); }}
          optionsSummary={`${genreLabel}${projectSettings.translationMode ? " · tradução" : ""}${projectSettings.useMemory ? " · memória" : ""}${glossary.length ? ` · glossário (${glossary.length})` : ""}`}
          options={<>
            <label className="field-label">GÊNERO (calibra metas e limiares)<select value={projectSettings.genre} onChange={(event) => setProjectSettings((current) => ({ ...current, genre: event.target.value as GenreId }))}>{catalog?.genres.map((item) => <option key={item.id} value={item.id}>{item.label}</option>)}</select></label>
            <label className="check"><input type="checkbox" checked={projectSettings.translationMode} onChange={(event) => setProjectSettings((current) => ({ ...current, translationMode: event.target.checked }))} /><span>É tradução EN→PT-BR (alerta suavizações e sugere fluidez)</span></label>
            {projectSettings.translationMode && <div className="original-source">
              {review.hasOriginal ? <><small>Original: {review.sourcePath} ({review.chapters.length} capítulos)</small>
                <label className="check"><input type="checkbox" checked={splitView} onChange={(event) => setSplitView(event.target.checked)} /><span>Tela dividida (original à esquerda)</span></label>
                <button className="secondary-button" type="button" onClick={review.clear}>Remover original</button></>
                : <><button className="secondary-button" type="button" onClick={() => void review.choose("epub")} disabled={review.loading}>{review.loading ? "Carregando…" : "Carregar original (EPUB)"}</button>
                  <button className="secondary-button" type="button" onClick={() => void review.choose("folder")} disabled={review.loading}>Carregar original (pasta)</button></>}
              {review.error && <small role="alert" style={{ color: "#a3404d" }}>{review.error}</small>}
            </div>}
            <label className="check"><input type="checkbox" checked={projectSettings.useMemory} onChange={(event) => setProjectSettings((current) => ({ ...current, useMemory: event.target.checked }))} /><span>Usar a memória do caderno nas análises que a pedem</span></label>
            <label className="check"><input type="checkbox" checked={projectSettings.adultContent} onChange={(event) => setProjectSettings((current) => ({ ...current, adultContent: event.target.checked }))} /><span>Contém cenas adultas (preservar intensidade, sem suavizar)</span></label>
            {glossary.length > 0 && <p className="glossary-chip">✓ glossário com {glossary.length} entradas enviado à IA</p>}
          </>}
        />
      </div>
      {isSettingsOpen && <AiSettingsDialog activeProviderId={providerId} models={models} availableModels={availableModels} keyStatus={keyStatus} onClose={() => setIsSettingsOpen(false)} onProviderChange={setProviderId} onModelChange={(id, model) => setModels((current) => ({ ...current, [id]: model }))} onModelsLoaded={(id, loadedModels) => setAvailableModels((current) => ({ ...current, [id]: loadedModels }))} onKeyStatusChange={updateKeyStatus} />}
      {isNotebookOpen && <NotebookDialog projectRoot={projectRoot} chapters={documents} activeChapterIndex={Math.max(activeIndex, 0)} activeChapterLines={translatedLines} ai={{ providerId, model: models[providerId], reasoningEffort: effortOptions?.configurable ? selectedEffort : null, ready: Boolean(keyStatus[providerId] && models[providerId]) }} genre={projectSettings.genre} glossaryLines={glossary} glossaryData={glossaryData} initialPath={notebookStart || undefined} onClose={() => setIsNotebookOpen(false)} onChanged={() => setMemoryNote("")} />}
      {resultsOpen && <ResultsPanel tab={resultsTab} onTab={setResultsTab} onClose={() => setResultsOpen(false)} summary={analysis} error={analysisError} findings={findings} suggestions={suggestions} notice={suggestionNotice} onApply={applySuggestion}
        extra={<>{memoryNote && <p className="privacy-note">{memoryNote}</p>}{genesis && <GenesisCard genesis={genesis} genreLabel={genreLabel} />}</>}
        diagnostics={<>{projectSettings.translationMode && review.report && <FidelityCard report={review.report} />}<ScanPanel text={chapterText} options={scanOptions} genreLabel={genreLabel} onApplyFluency={(fix) => applySuggestion({ before: fix.before, after: fix.after, reason: fix.reason })} /></>} />}
      {isBibleOpen && <BibleDialog projectRoot={projectRoot} chapters={documents} activeChapterIndex={Math.max(activeIndex, 0)} ai={{ providerId, model: models[providerId], reasoningEffort: effortOptions?.configurable ? selectedEffort : null, ready: Boolean(keyStatus[providerId] && models[providerId]) }} genre={projectSettings.genre} glossaryLines={glossary} onClose={() => setIsBibleOpen(false)} onChanged={() => setMemoryNote("")} />}
      {isReferencesOpen && activeDocument && <ReferencesPanel projectRoot={projectRoot} pageLines={pageReferenceLines} chapterLines={translatedLines} chapterName={activeDocument.name} onClose={() => setIsReferencesOpen(false)} onOpenInNotebook={(path) => { setNotebookStart(path); setIsNotebookOpen(true); }} onReplaceName={(name) => { setReplaceStart(name); void openReplace(); }} />}
      {isReplaceOpen && <ReplaceDialog initialFind={replaceStart} projectRoot={projectRoot} chapters={documents} activeChapterIndex={Math.max(activeIndex, 0)} onClose={() => { setIsReplaceOpen(false); setReplaceStart(""); }} onChapterFilesChanged={onChapterFilesChanged} onVaultChanged={() => { setMemoryNote(""); glossaryLoad(projectRoot).then((state) => { setGlossary(glossaryToLines(state.glossary)); setGlossaryData(state.glossary); }).catch(() => undefined); }} />}
      {isGlossaryOpen && <GlossaryDialog projectRoot={projectRoot} chapterPaths={documents.map((document) => document.path)} onClose={() => setIsGlossaryOpen(false)} onSaved={(saved: Glossary) => { setGlossary(glossaryToLines(saved)); setGlossaryData(saved); }} />}
      {isExportOpen && <ExportEpubDialog projectRoot={projectRoot} chapterPaths={documents.map((document) => document.path)} hasUnsavedChanges={Boolean(activeDocument) && editorText !== savedText} onClose={() => setIsExportOpen(false)} />}
      {pendingDocument && <PendingDocumentModal onCancel={() => setPendingDocument(null)} busy={isSaving}><span className="transition-icon">✦</span><div><span className="eyebrow">ALTERAÇÕES PENDENTES</span><h2 id="save-changes-title">Salvar edições e continuar?</h2><p>Você editou <strong>{activeDocument?.name}</strong>. O próximo capítulo será aberto somente depois de salvar ou descartar estas alterações.</p></div><div className="transition-actions"><button type="button" className="secondary-button" onClick={() => setPendingDocument(null)} disabled={isSaving}>Cancelar</button><button type="button" className="discard-button" onClick={discardAndContinue} disabled={isSaving}>Descartar e continuar</button><button type="button" className="primary-button" onClick={() => void saveAndContinue()} disabled={isSaving}>{isSaving ? "Salvando…" : "Salvar edições e continuar"}</button></div><small>Um backup do XHTML atual é criado automaticamente.</small></PendingDocumentModal>}
    </main>
  );
}
