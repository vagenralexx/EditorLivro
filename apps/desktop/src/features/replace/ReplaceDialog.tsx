import { useCallback, useEffect, useMemo, useState } from "react";
import { kindInfo, type NoteKind } from "../notebook/notebookApi";
import { replaceApply, replaceHistory, replacePreview, replaceUndo, type Applied, type HistoryItem, type Preview, type Sample } from "./replaceApi";
import "../settings/AiSettingsDialog.css";
import "./Replace.css";

interface ReplaceDialogProps {
  projectRoot: string;
  chapters: { path: string; name: string }[];
  activeChapterIndex: number;
  initialFind?: string;
  onClose: () => void;
  /** Capítulos (caminhos) que mudaram em disco: a tela recarrega o aberto. */
  onChapterFilesChanged: (paths: string[]) => void;
  /** Notas, glossário ou memória mudaram. */
  onVaultChanged: () => void;
}

const formsText = (forms: Record<string, number>) => Object.entries(forms).sort((a, b) => b[1] - a[1]).map(([form, n]) => `${form} ×${n}`).join(", ");

function SampleLine({ sample }: { sample: Sample }) {
  return <li className="rp-sample">…{sample.before} <del>{sample.matched}</del><ins>{sample.replacement}</ins> {sample.after}…</li>;
}

export function ReplaceDialog({ projectRoot, chapters, activeChapterIndex, initialFind = "", onClose, onChapterFilesChanged, onVaultChanged }: ReplaceDialogProps) {
  const [find, setFind] = useState(initialFind);
  const [replace, setReplace] = useState("");
  const [ignoreAccents, setIgnoreAccents] = useState(false);
  const [preserveCase, setPreserveCase] = useState(true);
  const [scope, setScope] = useState<"aberto" | "livro">("aberto");
  const [preview, setPreview] = useState<Preview | null>(null);
  const [chapterSel, setChapterSel] = useState<Set<string>>(new Set());
  const [noteSel, setNoteSel] = useState<Set<string>>(new Set());
  const [aliasSel, setAliasSel] = useState<Set<string>>(new Set());
  const [glossarySel, setGlossarySel] = useState(true);
  const [confirming, setConfirming] = useState(false);
  const [busy, setBusy] = useState("");
  const [error, setError] = useState("");
  const [result, setResult] = useState<Applied | null>(null);
  const [history, setHistory] = useState<HistoryItem[]>([]);
  const [undoConflict, setUndoConflict] = useState<{ id: string; files: string[] } | null>(null);
  const [info, setInfo] = useState("");

  const refreshHistory = useCallback(async () => { try { setHistory(await replaceHistory(projectRoot)); } catch { setHistory([]); } }, [projectRoot]);
  useEffect(() => { void refreshHistory(); }, [refreshHistory]);

  const paths = useMemo(() => (scope === "aberto" ? [chapters[activeChapterIndex]?.path].filter(Boolean) : chapters.map((chapter) => chapter.path)), [activeChapterIndex, chapters, scope]);
  const request = { root: projectRoot, paths, find, replace, ignoreAccents, preserveCase };
  const ready = find.trim() !== "" && replace.trim() !== "" && find.trim() !== replace.trim() && paths.length > 0;

  async function analyse() {
    setBusy("analisar"); setError(""); setInfo(""); setResult(null); setConfirming(false); setUndoConflict(null);
    try {
      const next = await replacePreview(request);
      setPreview(next);
      setChapterSel(new Set(next.chapters.filter((c) => c.count > 0).map((c) => c.path)));
      setNoteSel(new Set(next.notes.filter((n) => n.count > 0).map((n) => n.path)));
      setAliasSel(new Set(next.notes.filter((n) => n.entity).map((n) => n.path)));
      setGlossarySel(next.glossaryCount > 0);
    } catch (reason) { setPreview(null); setError(String(reason)); } finally { setBusy(""); }
  }

  // Mudar qualquer campo invalida a análise: o que foi mostrado já não vale.
  function edit<T>(setter: (value: T) => void) { return (value: T) => { setter(value); setPreview(null); setConfirming(false); }; }

  const toggle = (set: Set<string>, setSet: (next: Set<string>) => void, key: string) => { const next = new Set(set); if (next.has(key)) next.delete(key); else next.add(key); setSet(next); };

  const selectedChapterTotal = preview?.chapters.filter((c) => chapterSel.has(c.path)).reduce((sum, c) => sum + c.count, 0) ?? 0;
  const selectedNoteTotal = preview?.notes.filter((n) => noteSel.has(n.path)).reduce((sum, n) => sum + n.count, 0) ?? 0;
  const nothingSelected = chapterSel.size === 0 && noteSel.size === 0 && aliasSel.size === 0 && !(glossarySel && (preview?.glossaryCount ?? 0) > 0);

  async function apply() {
    if (!preview) return;
    setBusy("aplicar"); setError("");
    try {
      const applied = await replaceApply({ ...request, chapters: [...chapterSel], notes: [...noteSel], aliasNotes: [...aliasSel], glossary: glossarySel && preview.glossaryCount > 0 });
      setResult(applied); setPreview(null); setConfirming(false);
      onChapterFilesChanged(applied.changedChapters);
      if (applied.notesChanged.length || applied.glossaryChanged) onVaultChanged();
      await refreshHistory();
    } catch (reason) { setError(String(reason)); } finally { setBusy(""); }
  }

  async function undo(id: string, force: boolean) {
    setBusy("desfazer"); setError(""); setInfo("");
    try {
      const done = await replaceUndo(projectRoot, id, force);
      if (done.conflicts.length && !force) { setUndoConflict({ id, files: done.conflicts }); return; }
      setUndoConflict(null); setResult(null);
      setInfo(`Desfeito: ${done.restored.length} arquivo(s) restaurado(s).`);
      onChapterFilesChanged(done.changedChapters); onVaultChanged(); await refreshHistory();
    } catch (reason) { setError(String(reason)); } finally { setBusy(""); }
  }

  const canon = preview?.notes.filter((n) => n.canon || n.entity) ?? [];
  const plain = preview?.notes.filter((n) => !n.canon && !n.entity) ?? [];
  const noteRow = (note: Preview["notes"][number]) => (
    <li key={note.path} className="rp-note">
      <label><input type="checkbox" checked={noteSel.has(note.path)} disabled={note.count === 0} onChange={() => toggle(noteSel, setNoteSel, note.path)} /> <b>{note.title}</b> <small>{kindInfo[note.kind as NoteKind]?.label ?? note.kind} · {note.count} ocorrência(s)</small></label>
      {note.entity && <label className="rp-sub"><input type="checkbox" checked={aliasSel.has(note.path)} onChange={() => toggle(aliasSel, setAliasSel, note.path)} /> É a ficha deste nome: adicionar <b>{replace.trim()}</b> aos apelidos, para o app continuar reconhecendo o personagem nos capítulos</label>}
      {note.samples.length > 0 && noteSel.has(note.path) && <ul className="rp-samples">{note.samples.slice(0, 2).map((s, i) => <SampleLine key={i} sample={s} />)}</ul>}
    </li>
  );

  return (
    <div className="settings-backdrop" role="presentation" onMouseDown={onClose}>
      <section className="settings-dialog replace-dialog" role="dialog" aria-modal="true" aria-labelledby="replace-title" onMouseDown={(event) => event.stopPropagation()}>
        <div className="settings-title-row"><div><span className="eyebrow">REVISÃO</span><h2 id="replace-title">Substituir no livro</h2></div><button className="dialog-close" type="button" onClick={onClose} aria-label="Fechar">×</button></div>
        <p className="settings-description">Troca <b>palavras inteiras</b> (“Caio” não mexe em “Caiozinho”), só no texto — nunca em tags ou atributos do XHTML. A caixa acompanha o original: <i>Caio → Jordan</i>, <i>caio → jordan</i>. Você vê tudo antes, e dá para desfazer.</p>

        <div className="rp-form">
          <label className="settings-field">PROCURAR<input value={find} onChange={(event) => edit(setFind)(event.target.value)} placeholder="Caio" autoFocus /></label>
          <label className="settings-field">SUBSTITUIR POR<input value={replace} onChange={(event) => edit(setReplace)(event.target.value)} placeholder="Jordan" /></label>
        </div>
        <div className="rp-options">
          <label><input type="checkbox" checked={preserveCase} onChange={(event) => edit(setPreserveCase)(event.target.checked)} /> Acompanhar maiúsculas/minúsculas</label>
          <label><input type="checkbox" checked={ignoreAccents} onChange={(event) => edit(setIgnoreAccents)(event.target.checked)} /> Ignorar acentos (Iris = Íris)</label>
        </div>
        <div className="rp-scope" role="radiogroup" aria-label="Onde procurar">
          <label><input type="radio" name="scope" checked={scope === "aberto"} onChange={() => edit(setScope)("aberto")} /> Só o capítulo aberto{chapters[activeChapterIndex] ? ` (${chapters[activeChapterIndex].name})` : ""}</label>
          <label><input type="radio" name="scope" checked={scope === "livro"} onChange={() => edit(setScope)("livro")} /> O livro inteiro ({chapters.length} capítulos) <small>— é rápido: a análise lê os arquivos em segundos e nada é gravado até você confirmar</small></label>
        </div>
        <div className="settings-actions rp-analyse"><button className="primary-button" type="button" onClick={() => void analyse()} disabled={!ready || busy !== ""}>{busy === "analisar" ? "Analisando…" : "Analisar"}</button></div>

        {error && <p className="settings-feedback error" role="alert">{error}</p>}
        {info && <p className="settings-feedback success" role="status">{info}</p>}

        {preview && <div className="rp-result">
          {preview.total === 0 && preview.notes.length === 0 ? <p className="settings-feedback">Nenhuma ocorrência de palavra inteira “{find.trim()}” em {preview.scanned} capítulo(s){preview.partialTotal > 0 ? `; há ${preview.partialTotal} palavra(s) parecida(s) (${formsText(preview.partialWords)}) que não são o termo` : ""}.</p> : <>
            <p className="rp-summary"><b>{preview.total}</b> ocorrência(s) em <b>{preview.chapters.filter((c) => c.count > 0).length}</b> de {preview.scanned} capítulo(s) analisado(s).{preview.total > 0 && <> Formas encontradas: {formsText(preview.forms)}.</>}</p>
            {preview.partialTotal > 0 && <p className="rp-partial">⚠ {preview.partialTotal} palavra(s) <b>parecida(s)</b> serão ignoradas, por não serem a palavra inteira: {formsText(preview.partialWords)}. Se também devem mudar, rode outra substituição para elas.</p>}

            {preview.chapters.some((c) => c.count > 0) && <><h3>Capítulos</h3><ul className="rp-chapters">{preview.chapters.filter((c) => c.count > 0).map((chapter) => <li key={chapter.path}><label><input type="checkbox" checked={chapterSel.has(chapter.path)} onChange={() => toggle(chapterSel, setChapterSel, chapter.path)} /> <b>{chapter.name}</b> <small>{chapter.count} ocorrência(s){chapter.partial ? ` · ${chapter.partial} parecida(s) ignorada(s)` : ""}</small></label>
              {chapterSel.has(chapter.path) && <ul className="rp-samples">{chapter.samples.map((s, i) => <SampleLine key={i} sample={s} />)}</ul>}</li>)}</ul></>}

            {(canon.length > 0 || preview.glossaryCount > 0) && <div className="rp-story"><h3>Isso mexe na história?</h3>
              <p className="muted">Estas notas e dados do projeto citam “{find.trim()}”. Marque o que deve acompanhar a mudança; o que ficar desmarcado continua como está.</p>
              <ul>{canon.map(noteRow)}</ul>
              {preview.glossaryCount > 0 && <label className="rp-glossary"><input type="checkbox" checked={glossarySel} onChange={() => setGlossarySel(!glossarySel)} /> <b>Glossário</b> <small>{preview.glossaryCount} entrada(s) (personagens, apelidos, termos)</small></label>}</div>}
            {plain.length > 0 && <><h3>Outras notas e recaps</h3><ul>{plain.map(noteRow)}</ul></>}

            {!confirming ? <div className="settings-actions"><button className="primary-button" type="button" disabled={nothingSelected || busy !== ""} onClick={() => setConfirming(true)}>Revisar e aplicar…</button></div>
              : <div className="rp-confirm"><p><b>Confirmar:</b> substituir <b>{selectedChapterTotal}</b> ocorrência(s) em {chapterSel.size} capítulo(s){noteSel.size > 0 ? `, ${selectedNoteTotal} em ${noteSel.size} nota(s)` : ""}{glossarySel && preview.glossaryCount > 0 ? ", glossário" : ""}{aliasSel.size > 0 ? `, ${aliasSel.size} apelido(s) de ficha` : ""}. “{find.trim()}” → “{replace.trim()}”.</p>
                <p className="muted">Antes de gravar, os originais são copiados para <code>.livro/substituicoes</code> e há uma cópia de segurança do caderno. Capítulo aberto é salvo antes e recarregado depois.</p>
                <div className="settings-actions"><button className="secondary-button" type="button" onClick={() => setConfirming(false)}>Voltar</button><button className="primary-button" type="button" disabled={busy !== ""} onClick={() => void apply()}>{busy === "aplicar" ? "Aplicando…" : "Aplicar substituição"}</button></div></div>}
          </>}
        </div>}

        {result && <div className="settings-feedback success" role="status"><p><b>{result.replaced}</b> ocorrência(s) substituída(s): {result.changedChapters.length} capítulo(s){result.notesChanged.length ? `, ${result.notesChanged.length} nota(s)` : ""}{result.glossaryChanged ? ", glossário" : ""}{result.aliasesAdded.length ? `, apelidos em ${result.aliasesAdded.length} ficha(s)` : ""}.</p>{result.warnings.map((w) => <p key={w}>⚠ {w}</p>)}
          <button className="secondary-button" type="button" disabled={busy !== ""} onClick={() => void undo(result.id, false)}>Desfazer esta substituição</button></div>}

        {undoConflict && <div className="settings-feedback error" role="alert"><p>Estes arquivos foram editados depois da substituição, então nada foi restaurado: {undoConflict.files.join(", ")}.</p>
          <button className="secondary-button" type="button" onClick={() => void undo(undoConflict.id, true)}>Desfazer mesmo assim (perde as edições posteriores neles)</button></div>}

        {history.length > 0 && !result && <details className="rp-history"><summary>Substituições anteriores ({history.length})</summary><ul>{history.map((item) => <li key={item.id}><span>{item.createdAt.slice(0, 16).replace("T", " ")} · “{item.find}” → “{item.replace}” · {item.files} arquivo(s)</span><button className="secondary-button" type="button" disabled={busy !== ""} onClick={() => void undo(item.id, false)}>Desfazer</button></li>)}</ul></details>}
      </section>
    </div>
  );
}
