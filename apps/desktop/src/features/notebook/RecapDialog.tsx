import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useState, type ReactNode } from "react";
import { vaultApplyRecap, type Recap, type RecapChange } from "./notebookApi";
import "../settings/AiSettingsDialog.css";
import "./Notebook.css";
import { useDialogA11y } from "../../shared/useDialogA11y";

export interface AiConfig { providerId: string; model: string; reasoningEffort: string | null; ready: boolean; }

interface RecapDialogProps {
  projectRoot: string;
  chapterName: string;
  chapterLines: string[];
  order: number;
  ai: AiConfig;
  genre: string;
  glossary: string[];
  onClose: () => void;
  onSaved: () => void;
}

type Progress = { elapsedMs: number; answerChars: number; phase: "waiting" | "thinking" | "writing" };

function ListEditor({ title, items, render, onRemove }: { title: string; items: unknown[]; render: (index: number) => ReactNode; onRemove: (index: number) => void }) {
  if (!items.length) return null;
  return <div className="recap-group"><h3>{title} <small>({items.length})</small></h3>{items.map((_, index) => <div className="recap-item" key={index}>{render(index)}<button type="button" className="discard-button" aria-label="Remover item" onClick={() => onRemove(index)}>×</button></div>)}</div>;
}

export function RecapDialog({ projectRoot, chapterName, chapterLines, order, ai, genre, glossary, onClose, onSaved }: RecapDialogProps) {
  const [recap, setRecap] = useState<Recap | null>(null);
  const [busy, setBusy] = useState<"" | "generating" | "saving">("");
  const [progress, setProgress] = useState<Progress | null>(null);
  const [error, setError] = useState("");
  const [changes, setChanges] = useState<RecapChange[] | null>(null);

  useEffect(() => {
    if (busy !== "generating") return undefined;
    let active = true; let stop: (() => void) | undefined;
    listen<Progress>("ai-progress", (event) => setProgress(event.payload)).then((fn) => { if (active) stop = fn; else fn(); }).catch(() => undefined);
    return () => { active = false; stop?.(); };
  }, [busy]);

  async function generate() {
    setBusy("generating"); setError(""); setProgress(null);
    try {
      setRecap(await invoke<Recap>("generate_chapter_recap", { input: { root: projectRoot, provider: ai.providerId, model: ai.model, reasoningEffort: ai.reasoningEffort, chapterName, text: chapterLines.join("\n"), genre, glossary } }));
    } catch (reason) { setError(String(reason)); } finally { setBusy(""); }
  }

  async function save() {
    if (!recap) return;
    setBusy("saving"); setError("");
    try { setChanges(await vaultApplyRecap(projectRoot, chapterName.replace(/\.[^.]+$/u, ""), order, recap)); onSaved(); } catch (reason) { setError(String(reason)); } finally { setBusy(""); }
  }

  const update = (change: (current: Recap) => Recap) => setRecap((current) => (current ? change(current) : current));
  const removeAt = <K extends keyof Recap>(key: K, index: number) => update((current) => ({ ...current, [key]: (current[key] as unknown[]).filter((_, i) => i !== index) }));
  const setItem = <K extends "timeline" | "characters" | "places" | "threads_open" | "threads_closed">(key: K, index: number, patch: Partial<Recap[K][number]>) => update((current) => ({ ...current, [key]: (current[key] as object[]).map((item, i) => (i === index ? { ...item, ...patch } : item)) }) as Recap);
  const setText = (key: "facts" | "alerts", index: number, value: string) => update((current) => ({ ...current, [key]: current[key].map((item, i) => (i === index ? value : item)) }));

  const dialogRef = useDialogA11y<HTMLElement>(onClose);
  return (
    <div className="settings-backdrop recap-backdrop" role="presentation" onMouseDown={onClose}>
      <section className="settings-dialog recap-dialog" ref={dialogRef} tabIndex={-1} role="dialog" aria-modal="true" aria-labelledby="recap-title" onMouseDown={(event) => event.stopPropagation()}>
        <div className="settings-title-row"><div><span className="eyebrow">MEMÓRIA DO LIVRO</span><h2 id="recap-title">LOG-RECAP — {chapterName}</h2></div><button className="dialog-close" type="button" onClick={onClose} aria-label="Fechar">×</button></div>
        {!recap && !changes && <>
          <p className="settings-description">A IA lê o capítulo e propõe o resumo, a linha do tempo, o estado dos personagens, os fios abertos/fechados e os fatos estabelecidos. <b>Nada é gravado até você revisar e confirmar.</b> Personagens, lugares e fios novos viram notas no caderno, ligadas por [[links]].</p>
          {!ai.ready && <p className="settings-feedback error">Conecte um provedor e escolha um modelo em Configurações antes de gerar.</p>}
          {busy === "generating" && <p className="analysis-progress" role="status">{progress ? (progress.phase === "writing" ? `Escrevendo o recap… ${progress.answerChars} caracteres · ${Math.round(progress.elapsedMs / 1000)} s` : progress.phase === "thinking" ? `Modelo raciocinando… ${Math.round(progress.elapsedMs / 1000)} s` : `Aguardando o servidor (fila)… ${Math.round(progress.elapsedMs / 1000)} s`) : "Enviando ao servidor…"}<button type="button" className="discard-button" onClick={() => void invoke("cancel_ai_analysis")}>Cancelar</button></p>}
        </>}
        {recap && !changes && <div className="recap-body">
          <label className="settings-field">RESUMO<textarea rows={3} value={recap.summary} onChange={(event) => update((current) => ({ ...current, summary: event.target.value }))} /></label>
          <ListEditor title="Linha do tempo" items={recap.timeline} onRemove={(i) => removeAt("timeline", i)} render={(i) => <><input value={recap.timeline[i].event} onChange={(e) => setItem("timeline", i, { event: e.target.value })} /><input className="short" value={recap.timeline[i].scene} onChange={(e) => setItem("timeline", i, { scene: e.target.value })} placeholder="cena" /></>} />
          <ListEditor title="Personagens" items={recap.characters} onRemove={(i) => removeAt("characters", i)} render={(i) => <><input className="short" value={recap.characters[i].name} onChange={(e) => setItem("characters", i, { name: e.target.value })} /><input value={recap.characters[i].state} onChange={(e) => setItem("characters", i, { state: e.target.value })} /></>} />
          <ListEditor title="Lugares" items={recap.places} onRemove={(i) => removeAt("places", i)} render={(i) => <><input className="short" value={recap.places[i].name} onChange={(e) => setItem("places", i, { name: e.target.value })} /><input value={recap.places[i].note} onChange={(e) => setItem("places", i, { note: e.target.value })} /></>} />
          <ListEditor title="Fios abertos" items={recap.threads_open} onRemove={(i) => removeAt("threads_open", i)} render={(i) => <><input className="short" value={recap.threads_open[i].title} onChange={(e) => setItem("threads_open", i, { title: e.target.value })} /><input value={recap.threads_open[i].text} onChange={(e) => setItem("threads_open", i, { text: e.target.value })} /></>} />
          <ListEditor title="Fios fechados" items={recap.threads_closed} onRemove={(i) => removeAt("threads_closed", i)} render={(i) => <><input className="short" value={recap.threads_closed[i].title} onChange={(e) => setItem("threads_closed", i, { title: e.target.value })} /><input value={recap.threads_closed[i].text} onChange={(e) => setItem("threads_closed", i, { text: e.target.value })} /></>} />
          <ListEditor title="Fatos estabelecidos" items={recap.facts} onRemove={(i) => removeAt("facts", i)} render={(i) => <input value={recap.facts[i]} onChange={(e) => setText("facts", i, e.target.value)} />} />
          <ListEditor title="Alertas" items={recap.alerts} onRemove={(i) => removeAt("alerts", i)} render={(i) => <input value={recap.alerts[i]} onChange={(e) => setText("alerts", i, e.target.value)} />} />
        </div>}
        {changes && <div className="recap-body"><p className="settings-feedback success">Recap gravado no caderno.</p><ul className="change-list">{changes.map((change) => <li key={`${change.path}-${change.action}`}><b>{change.action}</b> {change.path}</li>)}</ul></div>}
        {error && <p className="settings-feedback error" role="alert">{error}</p>}
        <div className="settings-actions">
          <button className="secondary-button" type="button" onClick={onClose}>{changes ? "Fechar" : "Cancelar"}</button>
          {!recap && !changes && <button className="primary-button" type="button" onClick={() => void generate()} disabled={!ai.ready || busy !== "" || !chapterLines.length}>{busy === "generating" ? "Gerando…" : "Gerar proposta de recap"}</button>}
          {recap && !changes && <><button className="secondary-button" type="button" onClick={() => void generate()} disabled={busy !== ""}>Gerar de novo</button><button className="primary-button" type="button" onClick={() => void save()} disabled={busy !== ""}>{busy === "saving" ? "Gravando…" : "Gravar no caderno"}</button></>}
        </div>
      </section>
    </div>
  );
}
