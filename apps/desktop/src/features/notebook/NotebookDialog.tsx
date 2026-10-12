import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { Glossary } from "../skill/skillApi";
import { layoutGraph } from "./graphLayout";
import { renderMarkdown } from "./markdown";
import { kindInfo, kindOrder, splitFrontmatter, vaultCreate, vaultDelete, vaultIndex, vaultLocation, vaultMentions, vaultRead, vaultRename, vaultTimeline, vaultWrite, type NoteKind, type NoteMentions, type NoteMeta, type RecordSummary, type TimelineEntry, type VaultIndex } from "./notebookApi";
import { BibleDialog } from "./BibleDialog";
import { ProjectDataDialog } from "./ProjectDataDialog";
import { RecapDialog, type AiConfig } from "./RecapDialog";
import "../settings/AiSettingsDialog.css";
import "./Notebook.css";

interface NotebookDialogProps {
  projectRoot: string;
  chapters: { path: string; name: string }[];
  activeChapterIndex: number;
  activeChapterLines: string[];
  ai: AiConfig;
  genre: string;
  glossaryLines: string[];
  glossaryData: Glossary | null;
  /** Nota a abrir ao entrar (ex.: vindo do painel de referências). */
  initialPath?: string;
  onClose: () => void;
  onChanged: () => void;
}

type Tab = "notes" | "records" | "graph" | "timeline";
type StatusFilter = "todos" | "aberto" | "resolvido";

export function NotebookDialog({ projectRoot, chapters, activeChapterIndex, activeChapterLines, ai, genre, glossaryLines, glossaryData, initialPath, onClose, onChanged }: NotebookDialogProps) {
  const [index, setIndex] = useState<VaultIndex | null>(null);
  const [mentions, setMentions] = useState<NoteMentions[]>([]);
  const [timeline, setTimeline] = useState<TimelineEntry[]>([]);
  const [tab, setTab] = useState<Tab>("notes");
  const [selected, setSelected] = useState("");
  const [content, setContent] = useState("");
  const [saved, setSaved] = useState("");
  const [mode, setMode] = useState<"edit" | "preview">("preview");
  const [query, setQuery] = useState("");
  const [creating, setCreating] = useState<{ kind: NoteKind; title: string } | null>(null);
  const [renaming, setRenaming] = useState<string | null>(null);
  const [confirmDelete, setConfirmDelete] = useState(false);
  const [error, setError] = useState("");
  const [info, setInfo] = useState("");
  const [recapOpen, setRecapOpen] = useState(false);
  const [dataOpen, setDataOpen] = useState(false);
  const [bibleOpen, setBibleOpen] = useState(false);
  const [recordId, setRecordId] = useState("");
  const [recordPrefix, setRecordPrefix] = useState("");
  const [recordStatus, setRecordStatus] = useState<StatusFilter>("todos");
  const [recordQuery, setRecordQuery] = useState("");
  const [hiddenKinds, setHiddenKinds] = useState<Set<NoteKind>>(new Set());
  const [hover, setHover] = useState("");
  const textRef = useRef<HTMLTextAreaElement>(null);
  const dirty = content !== saved;

  const refresh = useCallback(async () => {
    try { const [idx, tl] = await Promise.all([vaultIndex(projectRoot), vaultTimeline(projectRoot)]); setIndex(idx); setTimeline(tl); } catch (reason) { setError(String(reason)); }
  }, [projectRoot]);

  useEffect(() => { void refresh(); }, [refresh]);
  useEffect(() => { if (initialPath) vaultRead(projectRoot, initialPath).then((text) => { setSelected(initialPath); setContent(text); setSaved(text); }).catch((reason) => setError(String(reason))); }, [initialPath, projectRoot]);
  // A chave evita recarregar quando o pai recria o array com o mesmo conteúdo.
  const chapterKey = chapters.map((chapter) => chapter.path).join("|");
  useEffect(() => { vaultMentions(projectRoot, chapterKey ? chapterKey.split("|") : []).then(setMentions).catch(() => setMentions([])); }, [projectRoot, chapterKey, index?.notes.length]);

  const notes = index?.notes ?? [];
  const byPath = useMemo(() => new Map(notes.map((note) => [note.path, note])), [notes]);
  const resolveMap = useMemo(() => {
    const map = new Map<string, string>();
    for (const note of notes) { map.set(note.title.toLowerCase(), note.path); map.set(note.path.replace(/\.md$/u, "").toLowerCase(), note.path); }
    for (const note of notes) for (const alias of note.aliases) if (!map.has(alias.toLowerCase())) map.set(alias.toLowerCase(), note.path);
    return map;
  }, [notes]);
  const resolve = useCallback((target: string) => resolveMap.get(target.trim().toLowerCase()) ?? resolveMap.get(target.trim().toLowerCase().split("/").pop() ?? "") ?? null, [resolveMap]);

  const save = useCallback(async () => {
    if (!selected || content === saved) return;
    try { await vaultWrite(projectRoot, selected, content); setSaved(content); await refresh(); onChanged(); } catch (reason) { setError(String(reason)); }
  }, [content, onChanged, projectRoot, refresh, saved, selected]);

  // Autosave 900 ms após parar de digitar.
  useEffect(() => {
    if (!dirty) return undefined;
    const timer = window.setTimeout(() => void save(), 900);
    return () => window.clearTimeout(timer);
  }, [dirty, save]);

  const open = useCallback(async (path: string) => {
    if (dirty) await save();
    setError(""); setConfirmDelete(false); setRenaming(null); setTab("notes");
    try { const text = await vaultRead(projectRoot, path); setSelected(path); setContent(text); setSaved(text); } catch (reason) { setError(String(reason)); }
  }, [dirty, projectRoot, save]);

  async function createNote(kind: NoteKind, title: string) {
    setError("");
    try { const path = await vaultCreate(projectRoot, kind, title); setCreating(null); await refresh(); onChanged(); await open(path); setMode("edit"); } catch (reason) { setError(String(reason)); }
  }

  async function createFromGlossary() {
    const characters = Object.entries(glossaryData?.personagens ?? {});
    let created = 0;
    for (const [name, data] of characters) {
      try {
        const path = await vaultCreate(projectRoot, "personagem", name);
        const text = await vaultRead(projectRoot, path);
        const aliases = [...data.apelidos, ...data.tratamentos].filter(Boolean);
        if (aliases.length) await vaultWrite(projectRoot, path, text.replace("aliases: []", `aliases: [${aliases.join(", ")}]`));
        created += 1;
      } catch { /* já existe: ignora */ }
    }
    setInfo(created ? `${created} personagem(ns) criado(s) a partir do glossário.` : "Nenhum personagem novo no glossário."); await refresh(); onChanged();
  }

  async function renameSelected(title: string) {
    try { const result = await vaultRename(projectRoot, selected, title); setRenaming(null); await refresh(); onChanged(); setInfo(`Renomeada. ${result.updatedNotes} nota(s) com links atualizados.`); const text = await vaultRead(projectRoot, result.newPath); setSelected(result.newPath); setContent(text); setSaved(text); } catch (reason) { setError(String(reason)); }
  }

  async function removeSelected() {
    try { await vaultDelete(projectRoot, selected); setSelected(""); setContent(""); setSaved(""); setConfirmDelete(false); await refresh(); onChanged(); } catch (reason) { setError(String(reason)); }
  }

  async function revealFolder() {
    try { await revealItemInDir((await vaultLocation(projectRoot)).path); } catch (reason) { setError(String(reason)); }
  }

  function insertLink(title: string) {
    const area = textRef.current;
    if (!area) return;
    const caret = area.selectionStart;
    const before = content.slice(0, caret).replace(/\[\[[^\]\n]*$/u, "");
    const next = `${before}[[${title}]]${content.slice(caret)}`;
    setContent(next);
    window.requestAnimationFrame(() => { area.focus(); const position = before.length + title.length + 4; area.setSelectionRange(position, position); });
  }

  const suggestion = useMemo(() => {
    const area = textRef.current;
    if (mode !== "edit" || !area) return [] as NoteMeta[];
    const match = /\[\[([^\]\n]*)$/u.exec(content.slice(0, area.selectionStart));
    if (!match) return [];
    const typed = match[1].toLowerCase();
    return notes.filter((note) => note.kind !== "capitulo" && note.title.toLowerCase().includes(typed)).slice(0, 6);
  }, [content, mode, notes]);

  const selectedMeta = byPath.get(selected);
  const backlinks = (index?.edges ?? []).filter((edge) => edge.to === selected).map((edge) => byPath.get(edge.from)).filter((note): note is NoteMeta => Boolean(note));
  const outgoing = (index?.edges ?? []).filter((edge) => edge.from === selected).map((edge) => byPath.get(edge.to)).filter((note): note is NoteMeta => Boolean(note));
  const broken = (index?.unresolved ?? []).filter((item) => item.from === selected);
  const myMentions = mentions.find((item) => item.path === selected);
  const filtered = notes.filter((note) => !query || `${note.title} ${note.heading ?? ""} ${note.aliases.join(" ")}`.toLowerCase().includes(query.toLowerCase()));

  // Grafo: nós = notas visíveis; arestas = links entre elas.
  const visibleNotes = notes.filter((note) => !hiddenKinds.has(note.kind));
  const visibleIds = useMemo(() => new Set(visibleNotes.map((note) => note.path)), [visibleNotes]);
  const graph = useMemo(() => {
    const edges = (index?.edges ?? []).filter((edge) => visibleIds.has(edge.from) && visibleIds.has(edge.to));
    const degree = new Map<string, number>();
    for (const edge of edges) { degree.set(edge.from, (degree.get(edge.from) ?? 0) + 1); degree.set(edge.to, (degree.get(edge.to) ?? 0) + 1); }
    const positioned = layoutGraph(visibleNotes.map((note) => ({ id: note.path, weight: degree.get(note.path) ?? 0 })), edges);
    return { edges, degree, positions: new Map(positioned.map((p) => [p.id, p])) };
  }, [index?.edges, visibleIds, visibleNotes]);
  const neighbors = useMemo(() => { const set = new Set<string>(); if (hover) { set.add(hover); for (const edge of graph.edges) { if (edge.from === hover) set.add(edge.to); if (edge.to === hover) set.add(edge.from); } } return set; }, [graph.edges, hover]);

  const records = index?.records ?? [];
  const recordIds = useMemo(() => new Set(records.map((record) => record.id)), [records]);
  const prefixes = useMemo(() => { const counts = new Map<string, number>(); for (const record of records) counts.set(record.prefix, (counts.get(record.prefix) ?? 0) + 1); return [...counts.entries()].sort(([a], [b]) => a.localeCompare(b)); }, [records]);
  const filteredRecords = records.filter((record) => (!recordPrefix || record.prefix === recordPrefix) && (recordStatus === "todos" || record.status === recordStatus) && (!recordQuery || `${record.id} ${record.title}`.toLowerCase().includes(recordQuery.toLowerCase())));
  const selectedRecord: RecordSummary | undefined = records.find((record) => record.id === recordId);
  const recordRows = (index?.recordRows ?? []).filter((row) => row.id === recordId);
  const openRecord = useCallback((id: string) => { setRecordId(id); setRecordPrefix(""); setRecordStatus("todos"); setRecordQuery(""); setTab("records"); }, []);
  const mdLinks = { resolve, onOpen: (path: string) => void open(path), onCreate: (target: string) => void createNote("nota", target), isRecord: (id: string) => recordIds.has(id), onRecord: openRecord };

  const empty = index !== null && notes.length === 0 && !creating;
  const activeChapter = chapters[activeChapterIndex];

  return (
    <div className="settings-backdrop notebook-backdrop" role="presentation" onMouseDown={() => { void save(); onClose(); }}>
      <section className="notebook" role="dialog" aria-modal="true" aria-labelledby="notebook-title" onMouseDown={(event) => event.stopPropagation()}>
        <header className="notebook-header">
          <div><span className="eyebrow">MEMÓRIA ENTRE CAPÍTULOS</span><h2 id="notebook-title">Caderno do livro</h2></div>
          <div className="notebook-tabs" role="tablist">{([["notes", "Notas"], ["records", `Registros${records.length ? ` (${records.length})` : ""}`], ["graph", "Grafo"], ["timeline", "Linha do tempo"]] as [Tab, string][]).map(([id, label]) => <button key={id} type="button" role="tab" aria-selected={tab === id} className={tab === id ? "active" : ""} onClick={() => setTab(id)}>{label}</button>)}</div>
          <div className="notebook-actions">
            <button className="secondary-button" type="button" onClick={() => { void save(); setRecapOpen(true); }} disabled={!activeChapter}>Gerar recap do capítulo aberto</button>
            <button className="secondary-button" type="button" onClick={() => void revealFolder()} disabled={!index?.exists} title="O caderno é uma pasta de arquivos .md compatível com o Obsidian">Mostrar pasta (Obsidian)</button>
            <button className="secondary-button" type="button" onClick={() => { void save(); setBibleOpen(true); }} title="Fatos canônicos do livro/série, por seção, com IA opcional">Bíblia</button>
            <button className="secondary-button" type="button" onClick={() => { void save(); setDataOpen(true); }} title="Onde os dados ficam, pasta do caderno e cópias de segurança">Dados do projeto</button>
            <button className="dialog-close" type="button" onClick={() => { void save(); onClose(); }} aria-label="Fechar caderno">×</button>
          </div>
        </header>
        {error && <p className="settings-feedback error" role="alert">{error} <button className="link-button" type="button" onClick={() => setError("")}>fechar</button></p>}
        {info && <p className="settings-feedback success" role="status">{info} <button className="link-button" type="button" onClick={() => setInfo("")}>ok</button></p>}

        {empty && <div className="notebook-empty"><h3>O caderno ainda está vazio</h3>
          <p>Aqui ficam personagens, lugares, fios narrativos e o <b>recap de cada capítulo</b>, ligados por <code>[[links]]</code>. A IA usa essa memória para checar continuidade sem precisar reler o livro.</p>
          <div className="empty-actions">
            <button className="primary-button" type="button" onClick={() => setRecapOpen(true)} disabled={!activeChapter}>Gerar recap do capítulo aberto</button>
            <button className="secondary-button" type="button" onClick={() => void createFromGlossary()} disabled={!Object.keys(glossaryData?.personagens ?? {}).length}>Criar personagens a partir do glossário</button>
            <button className="secondary-button" type="button" onClick={() => setCreating({ kind: "personagem", title: "" })}>Criar a primeira nota</button>
          </div></div>}

        {!empty && tab === "notes" && <div className="notebook-grid">
          <aside className="notebook-list">
            <input type="search" placeholder="Buscar nota ou apelido…" value={query} onChange={(event) => setQuery(event.target.value)} aria-label="Buscar notas" />
            {creating ? <form className="new-note" onSubmit={(event) => { event.preventDefault(); void createNote(creating.kind, creating.title); }}>
              <select value={creating.kind} onChange={(event) => setCreating({ ...creating, kind: event.target.value as NoteKind })}>{kindOrder.filter((kind) => kind !== "capitulo").map((kind) => <option key={kind} value={kind}>{kindInfo[kind].label}</option>)}</select>
              <input autoFocus value={creating.title} placeholder="Título da nota" onChange={(event) => setCreating({ ...creating, title: event.target.value })} />
              <div><button className="primary-button" type="submit" disabled={!creating.title.trim()}>Criar</button><button className="secondary-button" type="button" onClick={() => setCreating(null)}>Cancelar</button></div></form>
              : <button className="secondary-button" type="button" onClick={() => setCreating({ kind: "personagem", title: "" })}>＋ Nova nota</button>}
            <div className="notebook-groups">{kindOrder.map((kind) => { const group = filtered.filter((note) => note.kind === kind); if (!group.length) return null; return <details key={kind} open><summary><span className="kind-dot" style={{ background: kindInfo[kind].color }} />{kindInfo[kind].plural} <small>{group.length}</small></summary>
              <ul>{group.map((note) => <li key={note.path}><button type="button" className={note.path === selected ? "active" : ""} onClick={() => void open(note.path)} title={note.path}>{note.heading ?? note.title}{note.state === "fechado" && <small className="closed"> ✓ fechado</small>}</button></li>)}</ul></details>; })}</div>
          </aside>
          <main className="notebook-main">
            {!selectedMeta ? <p className="notebook-hint">Selecione uma nota à esquerda ou crie uma nova. Clique em um <span className="wikilink">[[link]]</span> para navegar entre as notas.</p> : <>
              <div className="note-head">
                {renaming !== null ? <form onSubmit={(event) => { event.preventDefault(); void renameSelected(renaming); }}><input autoFocus value={renaming} onChange={(event) => setRenaming(event.target.value)} /><button className="primary-button" type="submit">Renomear</button><button className="secondary-button" type="button" onClick={() => setRenaming(null)}>Cancelar</button></form>
                  : <><h3><span className="kind-dot" style={{ background: kindInfo[selectedMeta.kind].color }} />{selectedMeta.heading ?? selectedMeta.title}</h3><small>{kindInfo[selectedMeta.kind].label}{selectedMeta.aliases.length ? ` · também: ${selectedMeta.aliases.join(", ")}` : ""} · {dirty ? "salvando…" : "salvo"}</small></>}
                <div className="note-tools">
                  <button type="button" className={mode === "preview" ? "active" : ""} onClick={() => setMode("preview")}>Ler</button>
                  <button type="button" className={mode === "edit" ? "active" : ""} onClick={() => setMode("edit")}>Editar</button>
                  <button type="button" onClick={() => setRenaming(selectedMeta.title)}>Renomear</button>
                  {confirmDelete ? <><button type="button" className="danger" onClick={() => void removeSelected()}>Confirmar exclusão</button><button type="button" onClick={() => setConfirmDelete(false)}>Cancelar</button></> : <button type="button" onClick={() => setConfirmDelete(true)}>Excluir</button>}
                </div>
              </div>
              {mode === "edit" ? <div className="note-editor"><textarea ref={textRef} value={content} onChange={(event) => setContent(event.target.value)} spellCheck aria-label={`Conteúdo de ${selectedMeta.title}`} />
                {suggestion.length > 0 && <ul className="link-suggestions" aria-label="Sugestões de link">{suggestion.map((note) => <li key={note.path}><button type="button" onMouseDown={(event) => { event.preventDefault(); insertLink(note.title); }}><span className="kind-dot" style={{ background: kindInfo[note.kind].color }} />{note.title}</button></li>)}</ul>}
                <small className="note-help">Digite <code>[[</code> para linkar outra nota. O frontmatter (entre ---) guarda tipo, apelidos (<code>aliases</code>) e estado.</small></div>
                : <article className="note-preview">{renderMarkdown(content, mdLinks)}{!splitFrontmatter(content).body.trim() && <p className="notebook-hint">Nota vazia. Clique em Editar para escrever.</p>}</article>}
            </>}
          </main>
          <aside className="notebook-side">
            {selectedMeta ? <>
              <section><h4>Backlinks <small>({backlinks.length})</small></h4>{backlinks.length ? <ul>{backlinks.map((note) => <li key={note.path}><button type="button" onClick={() => void open(note.path)}>{note.title}</button></li>)}</ul> : <p className="muted">Nenhuma nota aponta para esta.</p>}</section>
              <section><h4>Links desta nota <small>({outgoing.length})</small></h4>{outgoing.length ? <ul>{outgoing.map((note) => <li key={note.path}><button type="button" onClick={() => void open(note.path)}>{note.title}</button></li>)}</ul> : <p className="muted">Sem links.</p>}{broken.length > 0 && <p className="muted broken-note">Links quebrados: {broken.map((item) => item.target).join(", ")}</p>}</section>
              {selectedMeta.kind !== "capitulo" && <section><h4>Citada nos capítulos <small>({myMentions?.total ?? 0}×)</small></h4>{myMentions ? <ul>{myMentions.chapters.map((entry) => <li key={entry.chapter}><span>{chapters[entry.chapter]?.name ?? `cap. ${entry.chapter + 1}`}</span> <b>{entry.count}×</b></li>)}</ul> : <p className="muted">Nenhuma menção ao título ou apelidos no texto.</p>}</section>}
            </> : <p className="muted">Backlinks, links e menções por capítulo aparecem aqui.</p>}
            {(index?.unresolved.length ?? 0) > 0 && <section><h4>Links sem nota <small>({new Set(index?.unresolved.map((u) => u.target)).size})</small></h4><ul>{[...new Set(index?.unresolved.map((u) => u.target))].slice(0, 12).map((target) => <li key={target}><button type="button" onClick={() => void createNote("nota", target)}>＋ {target}</button></li>)}</ul></section>}
          </aside>
        </div>}

        {tab === "records" && <div className="records-view">
          <div className="records-list">
            {records.length === 0 ? <p className="notebook-hint">Nenhum registro encontrado. Registros são linhas de tabela Markdown cuja primeira coluna é um ID estável (CF-01, FIO-03, LAC-02, PT-L1-01…), como nos seus Registro de Conflitos, Plot twists e Mapa de Fios.</p> : <>
              <div className="records-filters">
                <button type="button" className={recordPrefix === "" ? "active" : ""} onClick={() => setRecordPrefix("")}>Todos ({records.length})</button>
                {prefixes.map(([prefix, count]) => <button key={prefix} type="button" className={recordPrefix === prefix ? "active" : ""} onClick={() => setRecordPrefix(prefix)}>{prefix} ({count})</button>)}
                <select value={recordStatus} onChange={(event) => setRecordStatus(event.target.value as StatusFilter)} aria-label="Filtrar por estado"><option value="todos">Qualquer estado</option><option value="aberto">Abertos</option><option value="resolvido">Resolvidos</option></select>
                <input type="search" placeholder="Buscar ID ou texto…" value={recordQuery} onChange={(event) => setRecordQuery(event.target.value)} aria-label="Buscar registros" />
              </div>
              {filteredRecords.map((record) => <button key={record.id} type="button" className={`record-item ${record.id === recordId ? "active" : ""}`} onClick={() => setRecordId(record.id)}><b>{record.id}</b><span>{record.title || "—"}</span>{record.status ? <i className={`status-badge ${record.status}`} title="Estado inferido do texto da última linha; o arquivo não é alterado">{record.status}</i> : <i />}</button>)}
              {filteredRecords.length === 0 && <p className="muted">Nenhum registro com esses filtros.</p>}
            </>}
          </div>
          <aside className="record-detail">
            {!selectedRecord ? <p className="muted">Selecione um registro para ver todas as linhas em que ele aparece. O estado mostrado é <b>inferido</b> do texto (ex.: “resolvido”, “segue aberto”) e nunca é gravado nos seus arquivos.</p> : <>
              <h4>{selectedRecord.id}</h4>
              <p>{selectedRecord.status ? <i className={`status-badge ${selectedRecord.status}`}>{selectedRecord.status} (inferido)</i> : <span className="muted">estado não declarado</span>} · {selectedRecord.occurrences} linha(s)</p>
              {recordRows.map((row, i) => <div className="row-block" key={`${row.note}:${row.line}`}>
                <small className="muted">{i === recordRows.length - 1 && recordRows.length > 1 ? "Mais recente · " : ""}{row.note}, linha {row.line}</small>
                <dl>{row.headers.map((header, h) => row.cells[h] ? <div key={h}><dt>{header}</dt><dd>{renderMarkdown(row.cells[h], mdLinks)}</dd></div> : null)}</dl>
                <button type="button" className="secondary-button" onClick={() => { if (byPath.has(row.note)) void open(row.note); }} disabled={!byPath.has(row.note)}>Abrir nota</button>
              </div>)}
            </>}
          </aside>
        </div>}

        {!empty && tab === "graph" && <div className="graph-view">
          <div className="graph-filters">{kindOrder.map((kind) => { const count = notes.filter((note) => note.kind === kind).length; if (!count) return null; const off = hiddenKinds.has(kind); return <button key={kind} type="button" className={off ? "off" : ""} onClick={() => setHiddenKinds((current) => { const next = new Set(current); if (off) next.delete(kind); else next.add(kind); return next; })}><span className="kind-dot" style={{ background: kindInfo[kind].color }} />{kindInfo[kind].plural} ({count})</button>; })}</div>
          <svg viewBox="0 0 900 600" role="img" aria-label="Grafo de notas">
            {graph.edges.map((edge) => { const a = graph.positions.get(edge.from); const b = graph.positions.get(edge.to); if (!a || !b) return null; const lit = !hover || (neighbors.has(edge.from) && neighbors.has(edge.to)); return <line key={`${edge.from}>${edge.to}`} x1={a.x} y1={a.y} x2={b.x} y2={b.y} className={lit ? "edge" : "edge dim"} />; })}
            {visibleNotes.map((note) => { const p = graph.positions.get(note.path); if (!p) return null; const r = 6 + Math.min(graph.degree.get(note.path) ?? 0, 10) * 1.4; const lit = !hover || neighbors.has(note.path); return <g key={note.path} className={`node ${lit ? "" : "dim"} ${note.path === selected ? "selected" : ""}`} transform={`translate(${p.x},${p.y})`} onMouseEnter={() => setHover(note.path)} onMouseLeave={() => setHover("")} onClick={() => void open(note.path)} tabIndex={0} role="button" aria-label={note.title} onKeyDown={(event) => { if (event.key === "Enter") void open(note.path); }}><circle r={r} fill={kindInfo[note.kind].color} /><text y={r + 12} textAnchor="middle">{note.title.length > 22 ? `${note.title.slice(0, 21)}…` : note.title}</text></g>; })}
          </svg>
          <p className="muted">{visibleNotes.length} notas · {graph.edges.length} ligações. Passe o mouse para destacar vizinhos; clique para abrir. Notas sem links ficam isoladas — ligue-as com <code>[[ ]]</code>.</p>
        </div>}

        {!empty && tab === "timeline" && <div className="timeline-view">{timeline.length === 0 ? <p className="notebook-hint">A linha do tempo vem dos recaps dos capítulos. Gere o recap de um capítulo para começar.</p> : timeline.map((entry) => <section key={entry.chapter}><h4>{entry.chapter}</h4><div>{renderMarkdown(entry.events.map((event) => `- ${event}`).join("\n"), { ...mdLinks, onCreate: () => undefined })}</div></section>)}</div>}

        {bibleOpen && <BibleDialog projectRoot={projectRoot} chapters={chapters} activeChapterIndex={activeChapterIndex} ai={ai} genre={genre} glossaryLines={glossaryLines} onClose={() => { setBibleOpen(false); void refresh(); }} onChanged={() => { void refresh(); onChanged(); }} />}
        {dataOpen && <ProjectDataDialog projectRoot={projectRoot} onClose={() => setDataOpen(false)} onChanged={() => { setSelected(""); setContent(""); setSaved(""); setRecordId(""); void refresh(); onChanged(); }} />}
        {recapOpen && activeChapter && <RecapDialog projectRoot={projectRoot} chapterName={activeChapter.name} chapterLines={activeChapterLines} order={activeChapterIndex + 1} ai={ai} genre={genre} glossary={glossaryLines} onClose={() => setRecapOpen(false)} onSaved={() => { void refresh(); onChanged(); }} />}
      </section>
    </div>
  );
}
