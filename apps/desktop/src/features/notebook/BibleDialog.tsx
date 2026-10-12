import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useMemo, useState } from "react";
import { bibleAiDraft, bibleCatalog, bibleCreate, bibleList, bibleRead, bibleWriteSection, chaptersText, chunkChapters, levelInfo, type BibleDoc, type BibleInfo, type BibleLevel, type BibleSection, type CanonStatus, type SectionSpec } from "./bibleApi";
import type { AiConfig } from "./RecapDialog";
import "../settings/AiSettingsDialog.css";
import "./Notebook.css";
import { useDialogA11y } from "../../shared/useDialogA11y";

interface BibleDialogProps {
  projectRoot: string;
  chapters: { path: string; name: string }[];
  activeChapterIndex: number;
  ai: AiConfig;
  genre: string;
  glossaryLines: string[];
  onClose: () => void;
  onChanged: () => void;
}

type Progress = { elapsedMs: number; answerChars: number; phase: "waiting" | "thinking" | "writing" };
const BLOCK_CHARS = 40_000;
const statusLabel = (section: { body: string; status: CanonStatus | null }) => (!section.body.trim() ? "vazia" : section.status ?? "escrita");

export function BibleDialog({ projectRoot, chapters, activeChapterIndex, ai, genre, glossaryLines, onClose, onChanged }: BibleDialogProps) {
  const [catalog, setCatalog] = useState<SectionSpec[]>([]);
  const [bibles, setBibles] = useState<BibleInfo[] | null>(null);
  const [path, setPath] = useState("");
  const [doc, setDoc] = useState<BibleDoc | null>(null);
  const [selected, setSelected] = useState("");
  const [text, setText] = useState("");
  const [saved, setSaved] = useState("");
  const [creating, setCreating] = useState<{ level: BibleLevel; title: string; book: string } | null>(null);
  const [aiTab, setAiTab] = useState<"respostas" | "manuscrito">("respostas");
  const [notes, setNotes] = useState("");
  const [picked, setPicked] = useState<Set<number>>(new Set());
  const [proposal, setProposal] = useState<string | null>(null);
  const [busy, setBusy] = useState("");
  const [progress, setProgress] = useState<Progress | null>(null);
  const [error, setError] = useState("");
  const [info, setInfo] = useState("");

  const refreshList = useCallback(async () => { try { setBibles(await bibleList(projectRoot)); } catch (reason) { setError(String(reason)); setBibles([]); } }, [projectRoot]);
  useEffect(() => { bibleCatalog().then(setCatalog).catch((reason) => setError(String(reason))); void refreshList(); }, [refreshList]);
  useEffect(() => { if (!path && bibles?.length) setPath(bibles[0].path); }, [bibles, path]);
  useEffect(() => { setPicked(new Set([activeChapterIndex])); }, [activeChapterIndex]);

  const load = useCallback(async (target: string, keep?: string) => {
    if (!target) { setDoc(null); return; }
    try {
      const next = await bibleRead(projectRoot, target);
      setDoc(next);
      const first = keep ?? next.sections.find((s) => s.id)?.title ?? next.sections[0]?.title ?? "";
      setSelected(first);
    } catch (reason) { setError(String(reason)); }
  }, [projectRoot]);
  useEffect(() => { setProposal(null); void load(path); }, [load, path]);

  const missing = useMemo(() => (doc ? catalog.filter((spec) => spec.levels.includes(doc.level) && !doc.sections.some((s) => s.id === spec.id)) : []), [catalog, doc]);
  const missingPick = selected.startsWith("id:") ? missing.find((spec) => `id:${spec.id}` === selected) : undefined;
  const section: BibleSection | undefined = doc?.sections.find((s) => s.title === selected);
  const spec: SectionSpec | undefined = missingPick ?? catalog.find((s) => s.id === section?.id);
  const sectionTitle = missingPick?.title ?? section?.title ?? "";
  const dirty = text !== saved;

  useEffect(() => { setText(section?.body ?? ""); setSaved(section?.body ?? ""); setProposal(null); setNotes(""); }, [section?.title, section?.body, missingPick?.id]);

  useEffect(() => {
    if (!busy.startsWith("ia")) return undefined;
    let active = true; let stop: (() => void) | undefined;
    listen<Progress>("ai-progress", (event) => setProgress(event.payload)).then((fn) => { if (active) stop = fn; else fn(); }).catch(() => undefined);
    return () => { active = false; stop?.(); };
  }, [busy]);

  async function select(key: string) {
    if (dirty && !window.confirm("Há alterações não salvas nesta seção. Descartar?")) return;
    setSelected(key); setError(""); setInfo("");
  }

  async function createBible() {
    if (!creating) return;
    setBusy("criar"); setError("");
    try {
      const created = await bibleCreate(projectRoot, creating.level, creating.title, creating.level === "serie" ? null : creating.book || null);
      setCreating(null); await refreshList(); setPath(created); onChanged();
    } catch (reason) { setError(String(reason)); } finally { setBusy(""); }
  }

  async function write(body: string, status: CanonStatus | null, message: string) {
    if (!doc || !sectionTitle) return;
    setBusy("salvar"); setError(""); setInfo("");
    try {
      const key = missingPick ? missingPick.id : sectionTitle;
      const next = await bibleWriteSection(projectRoot, doc.path, key, body, status);
      setDoc(next);
      const title = missingPick ? next.sections.find((s) => s.id === missingPick.id)?.title ?? sectionTitle : sectionTitle;
      setSelected(title); setText(body); setSaved(body); setProposal(null); setInfo(message); onChanged();
    } catch (reason) { setError(String(reason)); } finally { setBusy(""); }
  }

  async function generate() {
    if (!doc || !sectionTitle) return;
    setBusy("ia"); setError(""); setProgress(null); setProposal(null);
    try {
      const base = { root: projectRoot, path: doc.path, section: missingPick ? missingPick.id : sectionTitle, provider: ai.providerId, model: ai.model, reasoningEffort: ai.reasoningEffort, genre, glossary: glossaryLines };
      if (aiTab === "respostas") {
        setProposal(await bibleAiDraft({ ...base, mode: "respostas", notes, current: text }));
      } else {
        const indexes = [...picked].sort((a, b) => a - b);
        const texts = await chaptersText(projectRoot, indexes.map((i) => chapters[i].path));
        const blocks = chunkChapters(indexes.map((i, n) => ({ name: chapters[i].name, text: texts[n] })), BLOCK_CHARS);
        let draft = text;
        for (let i = 0; i < blocks.length; i += 1) {
          setBusy(`ia ${i + 1}/${blocks.length}`); setProgress(null);
          draft = await bibleAiDraft({ ...base, mode: "manuscrito", text: blocks[i], current: draft });
        }
        setProposal(draft);
      }
    } catch (reason) { setError(String(reason)); } finally { setBusy(""); }
  }

  const requestClose = () => { if (!dirty || window.confirm("Descartar alterações não salvas?")) onClose(); };
  const dialogRef = useDialogA11y<HTMLElement>(requestClose);
  const aiBusy = busy.startsWith("ia");
  const blockInfo = busy.startsWith("ia ") ? ` — bloco ${busy.slice(3)}` : "";
  const progressText = progress ? (progress.phase === "writing" ? `Escrevendo… ${progress.answerChars} caracteres · ${Math.round(progress.elapsedMs / 1000)} s` : progress.phase === "thinking" ? `Modelo raciocinando… ${Math.round(progress.elapsedMs / 1000)} s` : `Aguardando o servidor… ${Math.round(progress.elapsedMs / 1000)} s`) : "Enviando ao servidor…";

  return (
    <div className="settings-backdrop notebook-backdrop" role="presentation" onMouseDown={() => { if (!dirty) onClose(); }}>
      <section className="notebook bible" ref={dialogRef} tabIndex={-1} role="dialog" aria-modal="true" aria-labelledby="bible-title" onMouseDown={(event) => event.stopPropagation()}>
        <header className="notebook-header">
          <div><span className="eyebrow">CONTINUIDADE EM NÍVEIS</span><h2 id="bible-title">Bíblia</h2></div>
          <div className="bible-picker">
            {bibles && bibles.length > 0 && <select value={path} onChange={(event) => { if (!dirty || window.confirm("Descartar alterações não salvas?")) setPath(event.target.value); }} aria-label="Bíblia aberta">{bibles.map((bible) => <option key={bible.path} value={bible.path}>{levelInfo[bible.level].label} · {bible.title}</option>)}</select>}
            <button className="secondary-button" type="button" onClick={() => setCreating({ level: "livro", title: "", book: "" })}>＋ Nova bíblia</button>
          </div>
          <button className="dialog-close" type="button" onClick={() => { if (!dirty || window.confirm("Descartar alterações não salvas?")) onClose(); }} aria-label="Fechar bíblia">×</button>
        </header>
        {error && <p className="settings-feedback error" role="alert">{error} <button className="link-button" type="button" onClick={() => setError("")}>fechar</button></p>}
        {info && <p className="settings-feedback success" role="status">{info} <button className="link-button" type="button" onClick={() => setInfo("")}>ok</button></p>}

        {creating && <form className="bible-new" onSubmit={(event) => { event.preventDefault(); void createBible(); }}>
          <h3>Nova bíblia</h3>
          <div className="bible-levels" role="radiogroup" aria-label="Nível">{(Object.keys(levelInfo) as BibleLevel[]).map((level) => <label key={level} className={creating.level === level ? "active" : ""}><input type="radio" name="level" checked={creating.level === level} onChange={() => setCreating({ ...creating, level })} /><b>{levelInfo[level].label}</b><small>{levelInfo[level].hint}</small></label>)}</div>
          <label className="settings-field">TÍTULO<input autoFocus value={creating.title} placeholder={creating.level === "serie" ? "Ex.: Sangue de Estrelas — Bíblia da Série" : "Ex.: Bíblia L1 — A Cura"} onChange={(event) => setCreating({ ...creating, title: event.target.value })} /></label>
          {creating.level !== "serie" && <label className="settings-field">LIVRO (opcional)<input value={creating.book} placeholder="Ex.: L1" onChange={(event) => setCreating({ ...creating, book: event.target.value })} /></label>}
          <p className="muted">Será criada como nota Markdown na pasta de bíblias do caderno, com as seções do nível. Nada existente é alterado.</p>
          <div className="settings-actions"><button className="secondary-button" type="button" onClick={() => setCreating(null)}>Cancelar</button><button className="primary-button" type="submit" disabled={!creating.title.trim() || busy === "criar"}>Criar bíblia</button></div>
        </form>}

        {!creating && bibles !== null && bibles.length === 0 && <div className="notebook-empty"><h3>Nenhuma bíblia no caderno ainda</h3>
          <p>A bíblia reúne os fatos canônicos do livro (linha do tempo, idades, personagens, regras, âncoras). Você pode escrever cada seção guiado por perguntas, pedir à IA para <b>estruturar suas anotações</b> ou para <b>extrair do manuscrito</b>. Se já tem uma bíblia em Markdown no seu vault, escolha a pasta em “Dados do projeto” e ela aparece aqui.</p>
          <div className="empty-actions"><button className="primary-button" type="button" onClick={() => setCreating({ level: "livro", title: "", book: "" })}>Criar a primeira bíblia</button></div></div>}

        {!creating && doc && <div className="bible-grid">
          <aside className="notebook-list bible-sections" aria-label="Seções da bíblia">
            <small className="muted">{levelInfo[doc.level].label}{doc.book ? ` · ${doc.book}` : ""} · {doc.path}</small>
            <ul>{doc.sections.map((s) => <li key={s.title}><button type="button" className={s.title === selected ? "active" : ""} onClick={() => void select(s.title)}>{s.title}<i className={`canon-chip ${statusLabel(s)}`}>{statusLabel(s)}</i></button></li>)}</ul>
            {missing.length > 0 && <><h3>Seções que faltam</h3><ul>{missing.map((m) => <li key={m.id}><button type="button" className={selected === `id:${m.id}` ? "active" : ""} onClick={() => void select(`id:${m.id}`)}>＋ {m.title}</button></li>)}</ul></>}
          </aside>

          <div className="notebook-main bible-main">
            {!sectionTitle ? <p className="notebook-hint">Escolha uma seção à esquerda.</p> : <>
              <div className="note-head"><h3>{sectionTitle}</h3><small>{section ? statusLabel(section) : "ainda não existe no arquivo"}{dirty ? " · alterações não salvas" : ""}</small></div>
              {spec ? <div className="bible-guide"><p><b>Para que serve:</b> {spec.purpose}</p><p><b>Formato:</b> {spec.format}</p><details><summary>Perguntas para guiar o preenchimento</summary><ul>{spec.questions.map((q) => <li key={q}>{q}</li>)}</ul></details></div> : <p className="muted">Seção livre do autor — o app só edita o corpo desta seção, sem tocar no resto do arquivo.</p>}
              <textarea className="bible-text" value={text} onChange={(event) => setText(event.target.value)} spellCheck aria-label={`Corpo da seção ${sectionTitle}`} placeholder="Escreva aqui em Markdown (tabelas, listas, [[links]] para fichas)…" />
              <div className="bible-actions">
                <button className="primary-button" type="button" disabled={busy !== "" || !text.trim()} onClick={() => void write(text, "confirmado", "Seção salva como canon confirmado.")}>Salvar e confirmar</button>
                <button className="secondary-button" type="button" disabled={busy !== "" || (!dirty && !missingPick)} onClick={() => void write(text, section?.status === "proposto" ? "proposto" : section?.status === "aberto" ? "aberto" : null, "Seção salva.")}>Salvar rascunho</button>
                <button className="secondary-button" type="button" disabled={busy !== ""} onClick={() => void write(text, "aberto", "Seção marcada como lacuna (#canon/aberto).")} title="Marca que falta definir algo; a memória do app trata como aberto">Marcar como lacuna</button>
              </div>

              <details className="bible-ai" open={Boolean(proposal) || aiBusy}>
                <summary>Gerar com IA</summary>
                {!ai.ready && <p className="settings-feedback error">Conecte um provedor e escolha um modelo em Configurações.</p>}
                <div className="notebook-tabs" role="tablist">{([["respostas", "A partir das minhas respostas"], ["manuscrito", "A partir do manuscrito"]] as const).map(([id, label]) => <button key={id} type="button" role="tab" aria-selected={aiTab === id} className={aiTab === id ? "active" : ""} onClick={() => setAiTab(id)}>{label}</button>)}</div>
                {aiTab === "respostas" ? <label className="settings-field">SUAS ANOTAÇÕES (livres, desorganizadas, como vier)<textarea rows={5} value={notes} onChange={(event) => setNotes(event.target.value)} placeholder="Responda as perguntas acima do seu jeito. A IA organiza no formato da seção sem inventar nada." /></label>
                  : <div className="bible-chapters"><p className="muted">Capítulos de onde extrair (a IA lê em blocos de ~{Math.round(BLOCK_CHARS / 1000)} mil caracteres e acumula o rascunho):</p><ul>{chapters.map((chapter, index) => <li key={chapter.path}><label><input type="checkbox" checked={picked.has(index)} onChange={() => setPicked((current) => { const next = new Set(current); if (next.has(index)) next.delete(index); else next.add(index); return next; })} /> {chapter.name}</label></li>)}</ul></div>}
                {aiBusy && <p className="analysis-progress" role="status">{progressText}{blockInfo}<button type="button" className="discard-button" onClick={() => void invoke("cancel_ai_analysis")}>Cancelar</button></p>}
                <div className="settings-actions"><button className="primary-button" type="button" onClick={() => void generate()} disabled={!ai.ready || busy !== "" || (aiTab === "respostas" ? !notes.trim() : picked.size === 0)}>{aiBusy ? "Gerando…" : "Gerar proposta"}</button></div>
                {proposal !== null && <div className="bible-proposal"><h4>Proposta da IA — revise e edite antes de aplicar</h4><textarea rows={10} value={proposal} onChange={(event) => setProposal(event.target.value)} aria-label="Proposta da IA" />
                  <p className="muted">Ao aplicar, a seção é gravada com <code>#canon/proposto</code>. Só vira canon quando você clicar em “Salvar e confirmar”. Uma cópia de segurança é feita antes.</p>
                  <div className="settings-actions"><button className="secondary-button" type="button" onClick={() => setProposal(null)}>Descartar</button><button className="secondary-button" type="button" onClick={() => { setText(proposal); setProposal(null); }}>Levar para o editor (sem gravar)</button><button className="primary-button" type="button" disabled={busy !== "" || !proposal.trim()} onClick={() => void write(proposal, "proposto", "Proposta aplicada como #canon/proposto. Revise e confirme.")}>Aplicar como proposta</button></div></div>}
              </details>
            </>}
          </div>
        </div>}
      </section>
    </div>
  );
}
