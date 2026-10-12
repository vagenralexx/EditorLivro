import { dirname, join } from "@tauri-apps/api/path";
import { open, save } from "@tauri-apps/plugin-dialog";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { fromStoredPath, loadProject, patchProject, toStoredPath } from "../project/projectStore";
import { copiesList, copiesRegister, detectWatermark, emptyEpubMetadata, epubBuild, epubInspect, epubValidate, type CopyRecord, type EpubBuildRequest, type EpubMetadata, type EpubProjectInfo, type EpubReport } from "./epubApi";
import "../settings/AiSettingsDialog.css";
import "./ExportEpubDialog.css";

interface ExportEpubDialogProps {
  projectRoot: string;
  /** Caminhos dos capítulos na ordem em que aparecem no painel do projeto. */
  chapterPaths: string[];
  hasUnsavedChanges: boolean;
  onClose: () => void;
}

interface StoredExport { metadata: Partial<EpubMetadata>; cover: string; }

const parseAuthors = (text: string) => text.split(/[;\n]/u).map((name) => name.trim()).filter(Boolean);
const safeFileName = (title: string) => (title.trim() || "livro").replace(/[\\/:*?"<>|]+/gu, " ").replace(/\s+/gu, " ").trim();
const fileName = (path: string) => path.split(/[\\/]/u).filter(Boolean).pop() ?? path;
const formatSize = (bytes: number) => bytes > 1048576 ? `${(bytes / 1048576).toFixed(1)} MB` : `${Math.max(1, Math.round(bytes / 1024))} KB`;

export function ExportEpubDialog({ projectRoot, chapterPaths, hasUnsavedChanges, onClose }: ExportEpubDialogProps) {
  const [info, setInfo] = useState<EpubProjectInfo | null>(null);
  const [form, setForm] = useState<EpubMetadata>(emptyEpubMetadata);
  const [authorsText, setAuthorsText] = useState("");
  const [cover, setCover] = useState("");
  const [report, setReport] = useState<EpubReport | null>(null);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState<"" | "loading" | "validating" | "building">("loading");
  const [result, setResult] = useState<{ path: string; size: number } | null>(null);
  const detected = useRef<EpubMetadata>(emptyEpubMetadata);
  const [wm, setWm] = useState({ enabled: false, recipient: "", visibleNotice: true, forensic: true });
  const [copies, setCopies] = useState<CopyRecord[]>([]);
  const [leak, setLeak] = useState<string>("");

  const buildRequest = useCallback((): EpubBuildRequest => {
    const authors = parseAuthors(authorsText);
    const metadata: Partial<EpubMetadata> = info?.mode === "extracted"
      // Em EPUB já extraído, só enviamos o que o usuário realmente alterou.
      ? Object.fromEntries(Object.entries({ ...form, authors }).filter(([key, value]) => JSON.stringify(value) !== JSON.stringify(detected.current[key as keyof EpubMetadata])))
      : { ...form, authors };
    return { root: projectRoot, metadata, chapters: chapterPaths, cover: info?.mode === "loose" && cover ? cover : null, watermark: wm.enabled ? { recipient: wm.recipient, visibleNotice: wm.visibleNotice, forensic: wm.forensic } : null };
  }, [authorsText, chapterPaths, cover, form, info?.mode, projectRoot, wm]);

  useEffect(() => { copiesList(projectRoot).then(setCopies).catch(() => setCopies([])); }, [projectRoot]);

  useEffect(() => {
    let active = true;
    Promise.all([epubInspect(projectRoot), loadProject(projectRoot)]).then(async ([found, project]) => {
      if (!active) return;
      const saved = project.file.epub;
      const stored: StoredExport | null = saved ? { metadata: (saved.metadados ?? {}) as Partial<EpubMetadata>, cover: saved.capa ? await fromStoredPath(projectRoot, saved.capa) : "" } : null;
      if (!active) return;
      const base = { ...emptyEpubMetadata, ...found.metadata, language: found.metadata.language || "pt-BR" };
      detected.current = base;
      const merged = found.mode === "loose" ? { ...base, ...stored?.metadata } : base;
      setInfo(found);
      setForm(merged);
      setAuthorsText(merged.authors.join("; "));
      setCover(found.mode === "loose" ? stored?.cover ?? "" : "");
    }).catch((reason) => active && setError(String(reason))).finally(() => active && setBusy(""));
    return () => { active = false; };
  }, [projectRoot]);

  const update = (field: keyof EpubMetadata, value: string) => { setForm((current) => ({ ...current, [field]: value })); setReport(null); setResult(null); };

  async function runValidate() {
    setBusy("validating"); setError(""); setResult(null);
    try { setReport(await epubValidate(buildRequest())); } catch (reason) { setError(String(reason)); } finally { setBusy(""); }
  }

  async function pickCover() {
    const selected = await open({ multiple: false, title: "Escolha a imagem da capa", filters: [{ name: "Imagem", extensions: ["jpg", "jpeg", "png", "webp", "gif"] }] });
    if (typeof selected === "string") { setCover(selected); setReport(null); setResult(null); }
  }

  async function runBuild() {
    setError(""); setResult(null);
    try {
      const target = await save({ title: "Salvar EPUB", defaultPath: await join(await dirname(projectRoot), `${safeFileName(form.title)}.epub`), filters: [{ name: "EPUB", extensions: ["epub"] }] });
      if (!target) return;
      setBusy("building");
      const response = await epubBuild(buildRequest(), target.toLowerCase().endsWith(".epub") ? target : `${target}.epub`);
      if (response.blockedReport) { setReport(response.blockedReport); return; }
      if (response.outcome) {
        setReport(response.outcome.report);
        setResult({ path: response.outcome.path, size: response.outcome.sizeBytes });
        void patchProject(projectRoot, "epub", { metadados: { ...form, authors: parseAuthors(authorsText), identifier: response.outcome.identifier }, capa: cover ? toStoredPath(projectRoot, cover) : "" }).catch(() => undefined);
        const mark = response.outcome.watermark;
        if (mark) setCopies(await copiesRegister(projectRoot, { token: mark.token, recipient: mark.recipient, title: form.title || "livro", createdAt: "", file: fileName(response.outcome.path) }));
        setForm((current) => ({ ...current, identifier: response.outcome?.identifier ?? current.identifier }));
      }
    } catch (reason) { setError(String(reason)); } finally { setBusy(""); }
  }

  async function checkLeak() {
    const selected = await open({ multiple: false, title: "Escolha o EPUB suspeito", filters: [{ name: "EPUB", extensions: ["epub"] }] });
    if (typeof selected !== "string") return;
    setLeak("Lendo…");
    try {
      const found = await detectWatermark(selected);
      if (!found) { setLeak("Nenhuma marca forense encontrada neste arquivo (ele não foi gerado com marca, ou a marca foi removida)."); return; }
      const owner = copies.find((copy) => copy.token === found.token);
      setLeak(owner ? `Código ${found.token}: exemplar gerado para ${owner.recipient} em ${owner.createdAt.slice(0, 10)} (${owner.file}). ${found.agreeing} de ${found.markers} marcas concordam.` : `Código ${found.token} encontrado (${found.agreeing} de ${found.markers} marcas), mas não consta no registro deste projeto.`);
    } catch (reason) { setLeak(String(reason)); }
  }

  const grouped = useMemo(() => ({
    error: report?.issues.filter((issue) => issue.severity === "error") ?? [],
    warning: report?.issues.filter((issue) => issue.severity === "warning") ?? [],
    info: report?.issues.filter((issue) => issue.severity === "info") ?? [],
  }), [report]);
  const loose = info?.mode === "loose";
  const disabled = busy !== "" || hasUnsavedChanges;

  return (
    <div className="settings-backdrop" role="presentation" onMouseDown={onClose}>
      <section className="settings-dialog export-dialog" role="dialog" aria-modal="true" aria-labelledby="export-title" onMouseDown={(event) => event.stopPropagation()}>
        <div className="settings-title-row"><div><span className="eyebrow">PUBLICAÇÃO</span><h2 id="export-title">Exportar EPUB</h2></div><button className="dialog-close" type="button" onClick={onClose} aria-label="Fechar exportação">×</button></div>
        {busy === "loading" && <p className="settings-description">Lendo a pasta do projeto…</p>}
        {info && <p className="settings-description">{loose
          ? `Pasta de capítulos (${info.chapterCount} arquivo${info.chapterCount === 1 ? "" : "s"}). O pacote EPUB 3 será gerado: sumário, NCX para Kindle, capa e folha de estilo profissional.`
          : `EPUB extraído detectado (EPUB ${info.epubVersion || "?"}, ${info.chapterCount} item(ns) na leitura). O conteúdo será reempacotado sem alterações; só os metadados que você mudar são atualizados.`}</p>}
        {hasUnsavedChanges && <p className="settings-feedback error" role="alert">Há alterações não gravadas no capítulo aberto. Salve (Ctrl + S) antes de exportar, ou elas ficarão fora do EPUB.</p>}
        <label className="settings-field">TÍTULO *<input value={form.title} onChange={(event) => update("title", event.target.value)} placeholder="Título do livro" /></label>
        <label className="settings-field">AUTOR(ES) — separe com ponto e vírgula<input value={authorsText} onChange={(event) => { setAuthorsText(event.target.value); setReport(null); setResult(null); }} placeholder="Nome Sobrenome; Outro Autor" /></label>
        <div className="export-row">
          <label className="settings-field">IDIOMA<input value={form.language} onChange={(event) => update("language", event.target.value)} placeholder="pt-BR" /></label>
          <label className="settings-field">EDITORA<input value={form.publisher} onChange={(event) => update("publisher", event.target.value)} /></label>
        </div>
        <label className="settings-field">SINOPSE<textarea value={form.description} onChange={(event) => update("description", event.target.value)} rows={3} /></label>
        {loose && <>
          <div className="export-row">
            <label className="settings-field">SÉRIE (opcional)<input value={form.series} onChange={(event) => update("series", event.target.value)} /></label>
            <label className="settings-field">Nº NA SÉRIE<input value={form.seriesIndex} onChange={(event) => update("seriesIndex", event.target.value)} inputMode="decimal" /></label>
          </div>
          <div className="export-cover"><span>CAPA</span><div><button className="secondary-button" type="button" onClick={() => void pickCover()} disabled={busy !== ""}>{cover ? "Trocar capa…" : "Escolher capa…"}</button>{cover && <><small title={cover}>{fileName(cover)}</small><button className="discard-button" type="button" onClick={() => { setCover(""); setReport(null); }}>Remover</button></>}</div></div>
        </>}
        <details className="watermark-box" open={wm.enabled}><summary>Proteção do exemplar (marca d'água)</summary>
          <p className="muted-note"><b>Isto não é DRM.</b> Não impede cópia: identifica a origem de um exemplar vazado. DRM de verdade (Adobe, Readium LCP, Kindle) é aplicado pela loja ou distribuidora, por exemplo a opção “Habilitar DRM” da KDP, e não por este aplicativo.</p>
          <label className="check-row"><input type="checkbox" checked={wm.enabled} onChange={(event) => { setWm({ ...wm, enabled: event.target.checked }); setReport(null); setResult(null); }} /> Marcar este exemplar</label>
          {wm.enabled && <>
            <label className="settings-field">EXEMPLAR PARA (nome, e-mail ou nº do pedido)<input value={wm.recipient} onChange={(event) => setWm({ ...wm, recipient: event.target.value })} placeholder="ana@exemplo.com" /></label>
            <label className="check-row"><input type="checkbox" checked={wm.visibleNotice} onChange={(event) => setWm({ ...wm, visibleNotice: event.target.checked })} /> Página de licença visível (nome + código), logo após a capa</label>
            <label className="check-row"><input type="checkbox" checked={wm.forensic} onChange={(event) => setWm({ ...wm, forensic: event.target.checked })} /> Marca forense invisível nos capítulos (caracteres de largura zero; leitores de tela e busca ignoram, mas ela pode ser removida por quem reescrever o texto)</label>
          </>}
          <div className="watermark-actions"><button className="secondary-button" type="button" onClick={() => void checkLeak()}>Verificar um EPUB suspeito…</button>{copies.length > 0 && <small>{copies.length} exemplar(es) registrado(s) em .livro-exemplares.json</small>}</div>
          {leak && <p className="settings-feedback success" role="status">{leak}</p>}
          {copies.length > 0 && <details><summary>Exemplares registrados</summary><ul className="issue-list info">{copies.map((copy) => <li key={copy.token}><b>{copy.token}</b> — {copy.recipient} <small>{copy.createdAt.slice(0, 10)} · {copy.file}</small></li>)}</ul></details>}
        </details>
        {error && <p className="settings-feedback error" role="alert">{error}</p>}
        {report && <div className="export-report" aria-live="polite">
          <strong>{report.errors ? `${report.errors} erro(s) impedem a geração` : "Nenhum erro encontrado"}{report.warnings ? ` · ${report.warnings} aviso(s)` : ""}</strong>
          {(["error", "warning"] as const).map((level) => grouped[level].length > 0 && <ul key={level} className={`issue-list ${level}`}>{grouped[level].map((issue, index) => <li key={`${issue.code}-${index}`}><b>{level === "error" ? "Erro" : "Aviso"}</b> {issue.message}{issue.file && <small>{issue.file}</small>}</li>)}</ul>)}
          {grouped.info.length > 0 && <details><summary>{grouped.info.length} observação(ões) automática(s)</summary><ul className="issue-list info">{grouped.info.map((issue, index) => <li key={`${issue.code}-${index}`}>{issue.message}{issue.file && <small>{issue.file}</small>}</li>)}</ul></details>}
        </div>}
        {result && <p className="settings-feedback success" role="status">EPUB gerado ({formatSize(result.size)}){wm.enabled && wm.recipient ? ` com marca para ${wm.recipient}` : ""}: {result.path} <button className="link-button" type="button" onClick={() => void revealItemInDir(result.path)}>Mostrar na pasta</button></p>}
        <div className="settings-actions">
          <button className="secondary-button" type="button" onClick={() => void runValidate()} disabled={disabled || !info}>{busy === "validating" ? "Validando…" : "Validar"}</button>
          <button className="primary-button" type="button" onClick={() => void runBuild()} disabled={disabled || !info}>{busy === "building" ? "Gerando…" : "Gerar EPUB…"}</button>
        </div>
      </section>
    </div>
  );
}
