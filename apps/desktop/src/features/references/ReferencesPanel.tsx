import { useCallback, useEffect, useMemo, useState } from "react";
import { renderMarkdown } from "../notebook/markdown";
import { kindInfo, vaultIndex, type VaultIndex } from "../notebook/notebookApi";
import { vaultEntity, vaultReferences, type EntityContext, type Reference } from "./referencesApi";
import "./References.css";

interface ReferencesPanelProps {
  projectRoot: string;
  /** Texto da página aberta e do capítulo inteiro, em parágrafos. */
  pageLines: string[];
  chapterLines: string[];
  chapterName: string;
  onClose: () => void;
  onOpenInNotebook: (path: string) => void;
  onReplaceName: (name: string) => void;
}

type Tab = "ficha" | "relacoes" | "registros" | "biblia";

export function ReferencesPanel({ projectRoot, pageLines, chapterLines, chapterName, onClose, onOpenInNotebook, onReplaceName }: ReferencesPanelProps) {
  const [scope, setScope] = useState<"pagina" | "capitulo">("pagina");
  const [refs, setRefs] = useState<Reference[] | null>(null);
  const [index, setIndex] = useState<VaultIndex | null>(null);
  const [trail, setTrail] = useState<string[]>([]);
  const [entity, setEntity] = useState<EntityContext | null>(null);
  const [tab, setTab] = useState<Tab>("ficha");
  const [error, setError] = useState("");

  const lines = scope === "pagina" ? pageLines : chapterLines;
  const text = useMemo(() => lines.join("\n"), [lines]);

  // Recalcula 500 ms depois que o texto para de mudar (o editor muda a cada tecla).
  useEffect(() => {
    let active = true;
    const timer = window.setTimeout(() => {
      vaultReferences(projectRoot, text ? text.split("\n") : []).then((found) => { if (active) { setRefs(found); setError(""); } }).catch((reason) => { if (active) setError(String(reason)); });
    }, 500);
    return () => { active = false; window.clearTimeout(timer); };
  }, [projectRoot, text]);

  useEffect(() => { vaultIndex(projectRoot).then(setIndex).catch(() => setIndex(null)); }, [projectRoot]);

  const resolveMap = useMemo(() => {
    const map = new Map<string, string>();
    for (const note of index?.notes ?? []) {
      map.set(note.title.toLowerCase(), note.path);
      map.set(note.path.replace(/\.md$/u, "").toLowerCase(), note.path);
      if (note.heading) map.set(note.heading.toLowerCase(), note.path);
    }
    for (const note of index?.notes ?? []) for (const alias of note.aliases) if (!map.has(alias.toLowerCase())) map.set(alias.toLowerCase(), note.path);
    return map;
  }, [index]);
  const resolve = useCallback((target: string) => { const t = target.trim().toLowerCase().replace(/^(\.\.?\/)+/u, ""); return resolveMap.get(t) ?? resolveMap.get(t.split("/").pop() ?? "") ?? null; }, [resolveMap]);

  const show = useCallback(async (path: string, push = true) => {
    try {
      const next = await vaultEntity(projectRoot, path);
      setEntity(next); setTab("ficha"); setError("");
      if (push) setTrail((current) => (current[current.length - 1] === path ? current : [...current, path]));
    } catch (reason) { setError(String(reason)); }
  }, [projectRoot]);

  function back() {
    setTrail((current) => {
      const next = current.slice(0, -1);
      const target = next[next.length - 1];
      if (target) void show(target, false); else setEntity(null);
      return next;
    });
  }

  const name = entity?.heading ?? entity?.title ?? "";
  const searchName = entity ? (entity.heading ?? entity.title.replace(/[-_]/gu, " ")).split(" ")[0] : "";
  const label = (note: { title: string; heading: string | null }) => note.heading ?? note.title;
  const links = { resolve, onOpen: (path: string) => void show(path), onCreate: () => undefined };

  return (
    <aside className="refs-panel" aria-label="Referências do caderno">
      <header>
        <div><span className="eyebrow">CADERNO</span><h2>Referências</h2></div>
        <button className="dialog-close" type="button" onClick={onClose} aria-label="Fechar referências">×</button>
      </header>
      {error && <p className="settings-feedback error" role="alert">{error}</p>}

      {!entity ? <>
        <div className="refs-scope" role="radiogroup" aria-label="Onde procurar">
          <button type="button" className={scope === "pagina" ? "active" : ""} onClick={() => setScope("pagina")}>Esta página</button>
          <button type="button" className={scope === "capitulo" ? "active" : ""} onClick={() => setScope("capitulo")} title={chapterName}>Capítulo</button>
        </div>
        {refs === null ? <p className="muted">Procurando no caderno…</p> : refs.length === 0 ? <p className="muted">Nenhum personagem, lugar ou fio do caderno é citado {scope === "pagina" ? "nesta página" : "neste capítulo"}. Cadastre-os no Caderno (ou crie a partir do glossário) para que apareçam aqui.</p> : <ul className="refs-list">{refs.map((ref) => <li key={ref.path}><button type="button" onClick={() => void show(ref.path)}><span className="kind-dot" style={{ background: kindInfo[ref.kind]?.color }} /><b>{ref.heading ?? ref.title}</b><small>{kindInfo[ref.kind]?.label} · {ref.count}×</small></button></li>)}</ul>}
        <p className="muted refs-hint">Clique em um nome para ver a ficha, as ligações, os registros (plots, conflitos, lacunas) e a bíblia em que ele aparece.</p>
      </> : <>
        <div className="refs-nav"><button type="button" className="link-button" onClick={back}>← {trail.length > 1 ? "voltar" : "lista"}</button><button type="button" className="link-button" onClick={() => { setEntity(null); setTrail([]); }}>lista</button></div>
        <h3 className="refs-title"><span className="kind-dot" style={{ background: kindInfo[entity.kind]?.color }} />{name}</h3>
        <small className="muted">{kindInfo[entity.kind]?.label}{entity.aliases.length ? ` · também: ${entity.aliases.join(", ")}` : ""} · <code>{entity.path}</code></small>
        <div className="refs-tabs" role="tablist">{([["ficha", "Ficha"], ["relacoes", `Relações ${entity.outgoing.length + entity.backlinks.length}`], ["registros", `Registros ${entity.records.length}`], ["biblia", `Bíblia e notas ${entity.sections.length + entity.notes.length}`]] as [Tab, string][]).map(([id, text]) => <button key={id} type="button" role="tab" aria-selected={tab === id} className={tab === id ? "active" : ""} onClick={() => setTab(id)}>{text}</button>)}</div>
        <div className="refs-body">
          {tab === "ficha" && <article className="note-preview">{renderMarkdown(entity.content, links)}</article>}
          {tab === "relacoes" && <>
            <h4>Esta nota liga-se a</h4>{entity.outgoing.length ? <ul>{entity.outgoing.map((link) => <li key={link.path}><button type="button" onClick={() => void show(link.path)}>{label(link)} <small>{kindInfo[link.kind]?.label}</small></button></li>)}</ul> : <p className="muted">Sem links de saída.</p>}
            <h4>Citada por (backlinks)</h4>{entity.backlinks.length ? <ul>{entity.backlinks.map((link) => <li key={link.path}><button type="button" onClick={() => void show(link.path)}>{label(link)} <small>{kindInfo[link.kind]?.label}</small></button></li>)}</ul> : <p className="muted">Nenhuma nota aponta para esta.</p>}</>}
          {tab === "registros" && (entity.records.length ? <ul className="refs-records">{entity.records.map((rec) => <li key={`${rec.id}-${rec.note}-${rec.line}`}><div><b>{rec.id}</b>{rec.status && <i className={`status-badge ${rec.status}`} title="Estado inferido do texto">{rec.status}</i>}</div><p>{rec.snippet || rec.title}</p><button type="button" className="link-button" onClick={() => { const target = resolve(rec.note.replace(/\.md$/u, "")); if (target) void show(target); }}>{rec.note}, linha {rec.line}</button></li>)}</ul> : <p className="muted">Nenhum registro (CF-, FIO-, LAC-, PT-…) cita este nome.</p>)}
          {tab === "biblia" && <>
            {entity.sections.length > 0 && <><h4>Seções que citam</h4><ul className="refs-records">{entity.sections.map((sec) => <li key={`${sec.note}#${sec.heading}`}><div><b>{sec.heading}</b>{sec.status && <i className={`canon-chip ${sec.status}`}>{sec.status}</i>}</div><p>{sec.snippet}</p><button type="button" className="link-button" onClick={() => { const target = resolve(sec.note.replace(/\.md$/u, "")); if (target) void show(target); }}>{sec.noteTitle}</button></li>)}</ul></>}
            {entity.notes.length > 0 && <><h4>Notas que a citam</h4><ul>{entity.notes.map((note) => <li key={note.path}><button type="button" onClick={() => void show(note.path)}>{label(note)} <small>{kindInfo[note.kind]?.label} · {note.count}×</small></button></li>)}</ul></>}
            {entity.sections.length === 0 && entity.notes.length === 0 && <p className="muted">Mais nenhuma nota ou seção cita este nome.</p>}</>}
        </div>
        <footer className="refs-actions"><button className="secondary-button" type="button" onClick={() => onOpenInNotebook(entity.path)}>Abrir no Caderno</button><button className="secondary-button" type="button" onClick={() => onReplaceName(searchName)} title="Trocar este nome no livro">Substituir nome…</button></footer>
      </>}
    </aside>
  );
}
