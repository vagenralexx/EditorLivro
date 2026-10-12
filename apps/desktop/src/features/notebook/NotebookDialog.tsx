import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { Glossary } from "../skill/skillApi";
import { GraphView } from "./GraphView";
import { renderMarkdown } from "./markdown";
import { kindInfo, kindOrder, splitFrontmatter, vaultCreate, vaultDelete, vaultIndex, vaultLocation, vaultMentions, vaultRead, vaultRename, vaultTimeline, vaultWrite, type NoteKind, type NoteMentions, type NoteMeta, type RecordSummary, type TimelineEntry, type VaultIndex } from "./notebookApi";
import { BibleDialog } from "./BibleDialog";
import { ProjectDataDialog } from "./ProjectDataDialog";
import { RecapDialog, type AiConfig } from "./RecapDialog";
import "../settings/AiSettingsDialog.css";
import "./Notebook.css";
import { useDialogA11y } from "../../shared/useDialogA11y";

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
const RECORD_LIST_LIMIT = 300;

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
  const [userOpen, setUserOpen] = useState<Partial<Record<NoteKind, boolean>>>({});
  const [recordGroup, setRecordGroup] = useState<"ids" | "nomes">("ids");
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

  // Ctrl+S salva a nota na hora (o autosave abaixo cobre o resto).
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => { if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "s") { event.preventDefault(); void save(); } };
    document.addEventListener("keydown", onKey, true);
    return () => document.removeEventListener("keydown", onKey, true);
  }, [save]);

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

  const allRecords = index?.records ?? [];
  const idRecords = allRecords.filter((record) => !record.keyed);
  const namedRecords = allRecords.filter((record) => record.keyed);
  const records = recordGroup === "ids" ? idRecords : namedRecords;
  const recordIds = useMemo(() => new Set(records.filter((record) => !record.keyed).map((record) => record.id)), [records]);
  const prefixes = useMemo(() => { const counts = new Map<string, number>(); for (const record of records) counts.set(record.prefix, (counts.get(record.prefix) ?? 0) + 1); return [...counts.entries()].sort(([a], [b]) => a.localeCompare(b)); }, [records]);
  const filteredRecords = records.filter((record) => (!recordPrefix || record.prefix === recordPrefix) && (recordStatus === "todos" || record.status === recordStatus) && (!recordQuery || `${record.id} ${record.title}`.toLowerCase().includes(recordQuery.toLowerCase())));
  const selectedRecord: RecordSummary | undefined = records.find((record) => record.uid === recordId);
  const recordRows = (index?.recordRows ?? []).filter((row) => row.uid === recordId);
  const openRecord = useCallback((id: string) => { setRecordId(records.find((record) => record.id === id && !record.keyed)?.uid ?? id); setRecordPrefix(""); setRecordStatus("todos"); setRecordQuery(""); setTab("records"); }, [records]);
  const mdLinks = { resolve, onOpen: (path: string) => void open(path), onCreate: (target: string) => void createNote("nota", target), isRecord: (id: string) => recordIds.has(id), onRecord: openRecord };

  const empty = index !== null && notes.length === 0 && !creating;
  const activeChapter = chapters[activeChapterIndex];

  const dialogRef = useDialogA11y<HTMLElement>(() => { void save(); onClose(); });
  return (
    <div className="settings-backdrop notebook-backdrop" role="presentation" onMouseDown={() => { void save(); onClose(); }}>
      <section className="notebook" ref={dialogRef} tabIndex={-1} role="dialog" aria-modal="true" aria-labelledby="notebook-title" onMouseDown={(event) => event.stopPropagation()}>
        <header className="notebook-header">
          <div><span className="eyebrow">MEMÓRIA ENTRE CAPÍTULOS</span><h2 id="notebook-title">Caderno do livro</h2></div>
          <div className="notebook-tabs" role="tablist">{([["notes", "Notas"], ["records", `Registros${idRecords.length || namedRecords.length ? ` (${idRecords.length || namedRecords.length})` : ""}`], ["graph", "Grafo"], ["timeline", "Linha do tempo"]] as [Tab, string][]).map(([id, label]) => <button key={id} type="button" role="tab" aria-selected={tab === id} className={tab === id ? "active" : ""} onClick={() => setTab(id)}>{label}</button>)}</div>
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
          <aside className="notebook-list" aria-label="Notas do caderno">
            <input type="search" placeholder="Buscar nota ou apelido…" value={query} onChange={(event) => setQuery(event.target.value)} aria-label="Buscar notas" />
            {creating ? <form className="new-note" onSubmit={(event) => { event.preventDefault(); void createNote(creating.kind, creating.title); }}>
              <select value={creating.kind} onChange={(event) => setCreating({ ...creating, kind: event.target.value as NoteKind })}>{kindOrder.filter((kind) => kind !== "capitulo").map((kind) => <option key={kind} value={kind}>{kindInfo[kind].label}</option>)}</select>
              <input autoFocus value={creating.title} placeholder="Título da nota" onChange={(event) => setCreating({ ...creating, title: event.target.value })} />
              <div><button className="primary-button" type="submit" disabled={!creating.title.trim()}>Criar</button><button className="secondary-button" type="button" onClick={() => setCreating(null)}>Cancelar</button></div></form>
              : <button className="secondary-button" type="button" onClick={() => setCreating({ kind: "personagem", title: "" })}>＋ Nova nota</button>}
            <div className="notebook-groups">{kindOrder.map((kind) => { const group = filtered.filter((note) => note.kind === kind); if (!group.length) return null; return <details key={kind} open={userOpen[kind] ?? (kind === selectedMeta?.kind || query.trim() !== "")} onToggle={(event) => { const open = event.currentTarget.open; setUserOpen((current) => (current[kind] === open ? current : { ...current, [kind]: open })); }}><summary><span className="kind-dot" style={{ background: kindInfo[kind].color }} />{kindInfo[kind].plural} <small>{group.length}</small></summary>
              <ul>{group.map((note) => <li key={note.path}><button type="button" className={note.path === selected ? "active" : ""} onClick={() => void open(note.path)} title={note.path}>{note.heading ?? note.title}{note.state === "fechado" && <small className="closed"> ✓ fechado</small>}</button></li>)}</ul></details>; })}</div>
          </aside>
          <div className="notebook-main">
            {!selectedMeta ? <p className="notebook-hint">Selecione uma nota à esquerda ou crie uma nova. Clique em um <span className="wikilink">[[link]]</span> para navegar entre as notas.</p> : <>
              <div className="note-head">
                {renaming !== null ? <form onSubmit={(event) => { event.preventDefault(); void renameSelected(renaming); }}><input autoFocus value={renaming} onChange={(event) => setRenaming(event.target.value)} /><button className="primary-button" type="submit">Renomear</button><button className="secondary-button" type="button" onClick={() => setRenaming(null)}>Cancelar</button></form>
                  : <><h3><span className="kind-dot" style={{ background: kindInfo[selectedMeta.kind].color }} />{selectedMeta.heading ?? selectedMeta.title}</h3><small>{kindInfo[selectedMeta.kind].label}{selectedMeta.aliases.length ? ` · também: ${selectedMeta.aliases.join(", ")}` : ""} · <span className={dirty ? "save-state dirty" : "save-state"} role="status">{dirty ? "alterações ainda não salvas" : "✓ salvo"}</span></small></>}
                <div className="note-tools">
                  <button type="button" className="save-now" onClick={() => void save()} disabled={!dirty} title="Salvar agora (Ctrl+S)">Salvar</button>
                  <button type="button" className={mode === "preview" ? "active" : ""} onClick={() => setMode("preview")}>Ler</button>
                  <button type="button" className={mode === "edit" ? "active" : ""} onClick={() => setMode("edit")}>Editar</button>
                  <button type="button" onClick={() => setRenaming(selectedMeta.title)}>Renomear</button>
                  {confirmDelete ? <><button type="button" className="danger" onClick={() => void removeSelected()}>Confirmar exclusão</button><button type="button" onClick={() => setConfirmDelete(false)}>Cancelar</button></> : <button type="button" onClick={() => setConfirmDelete(true)}>Excluir</button>}
                </div>
              </div>
              {mode === "edit" ? <div className="note-editor"><textarea ref={textRef} value={content} onChange={(event) => setContent(event.target.value)} spellCheck aria-label={`Conteúdo de ${selectedMeta.title}`} />
                {suggestion.length > 0 && <ul className="link-suggestions" aria-label="Sugestões de link">{suggestion.map((note) => <li key={note.path}><button type="button" onMouseDown={(event) => { event.preventDefault(); insertLink(note.title); }}><span className="kind-dot" style={{ background: kindInfo[note.kind].color }} />{note.title}</button></li>)}</ul>}
                <small className="note-help">Salva sozinho 1 s depois que você para de digitar; <kbd>Ctrl</kbd>+<kbd>S</kbd> salva na hora. Digite <code>[[</code> para linkar outra nota. O frontmatter (entre ---) guarda tipo, apelidos (<code>aliases</code>) e estado.</small></div>
                : <article className="note-preview">{renderMarkdown(content, mdLinks)}{!splitFrontmatter(content).body.trim() && <p className="notebook-hint">Nota vazia. Clique em Editar para escrever.</p>}</article>}
            </>}
          </div>
          <aside className="notebook-side" aria-label="Ligações da nota">
            {selectedMeta ? <>
              <section><h3>Backlinks <small>({backlinks.length})</small></h3>{backlinks.length ? <ul>{backlinks.map((note) => <li key={note.path}><button type="button" onClick={() => void open(note.path)}>{note.title}</button></li>)}</ul> : <p className="muted">Nenhuma nota aponta para esta.</p>}</section>
              <section><h3>Links desta nota <small>({outgoing.length})</small></h3>{outgoing.length ? <ul>{outgoing.map((note) => <li key={note.path}><button type="button" onClick={() => void open(note.path)}>{note.title}</button></li>)}</ul> : <p className="muted">Sem links.</p>}{broken.length > 0 && <p className="muted broken-note">Links quebrados: {broken.map((item) => item.target).join(", ")}</p>}</section>
              {selectedMeta.kind !== "capitulo" && <section><h3>Citada nos capítulos <small>({myMentions?.total ?? 0}×)</small></h3>{myMentions ? <ul>{myMentions.chapters.map((entry) => <li key={entry.chapter}><span>{chapters[entry.chapter]?.name ?? `cap. ${entry.chapter + 1}`}</span> <b>{entry.count}×</b></li>)}</ul> : <p className="muted">Nenhuma menção ao título ou apelidos no texto.</p>}</section>}
            </> : <p className="muted">Backlinks, links e menções por capítulo aparecem aqui.</p>}
            {(index?.unresolved.length ?? 0) > 0 && <section><h3>Links sem nota <small>({new Set(index?.unresolved.map((u) => u.target)).size})</small></h3><ul>{[...new Set(index?.unresolved.map((u) => u.target))].slice(0, 12).map((target) => <li key={target}><button type="button" onClick={() => void createNote("nota", target)}>＋ {target}</button></li>)}</ul></section>}
          </aside>
        </div>}

        {tab === "records" && <div className="records-view">
          <div className="records-list">
            {allRecords.length === 0 ? <p className="notebook-hint">Nenhum registro encontrado. Registros são linhas de tabela Markdown cuja primeira coluna é um ID estável (CF-01, FIO-03, LAC-02, PT-L1-01…) ou, nas notas do tipo Registro/Mapa, o nome do item (Mapa de Âncoras, Matriz de Relações, Verificação de Pontas Soltas).</p> : <>
              <div className="records-groups" role="group" aria-label="Tipo de registro">
                <button type="button" aria-pressed={recordGroup === "ids"} className={recordGroup === "ids" ? "active" : ""} onClick={() => { setRecordGroup("ids"); setRecordPrefix(""); }}>Com ID ({idRecords.length})</button>
                <button type="button" aria-pressed={recordGroup === "nomes"} className={recordGroup === "nomes" ? "active" : ""} onClick={() => { setRecordGroup("nomes"); setRecordPrefix(""); }} title="Linhas de tabelas de mapas e matrizes, identificadas pelo nome do item">Tabelas por nome ({namedRecords.length})</button>
              </div>
              <div className="records-filters">
                {recordGroup === "ids"
                  ? <><button type="button" className={recordPrefix === "" ? "active" : ""} onClick={() => setRecordPrefix("")}>Todos ({records.length})</button>
                    {prefixes.map(([prefix, count]) => <button key={prefix} type="button" className={recordPrefix === prefix ? "active" : ""} onClick={() => setRecordPrefix(prefix)}>{prefix} ({count})</button>)}</>
                  : <select value={recordPrefix} onChange={(event) => setRecordPrefix(event.target.value)} aria-label="Filtrar por nota de origem"><option value="">Todas as notas ({records.length})</option>{prefixes.map(([prefix, count]) => <option key={prefix} value={prefix}>{prefix} ({count})</option>)}</select>}
                <select value={recordStatus} onChange={(event) => setRecordStatus(event.target.value as StatusFilter)} aria-label="Filtrar por estado"><option value="todos">Qualquer estado</option><option value="aberto">Abertos</option><option value="resolvido">Resolvidos</option></select>
                <input type="search" placeholder="Buscar ID ou texto…" value={recordQuery} onChange={(event) => setRecordQuery(event.target.value)} aria-label="Buscar registros" />
              </div>
              {filteredRecords.slice(0, RECORD_LIST_LIMIT).map((record) => <button key={record.uid} type="button" className={`record-item ${record.uid === recordId ? "active" : ""} ${record.keyed ? "keyed" : ""}`} onClick={() => setRecordId(record.uid)}><b>{record.id}</b><span>{record.title || "—"}</span>{record.status ? <i className={`status-badge ${record.status}`} title="Estado inferido do texto da última linha; o arquivo não é alterado">{record.status}</i> : <i />}</button>)}
              {filteredRecords.length > RECORD_LIST_LIMIT && <p className="muted">Mostrando {RECORD_LIST_LIMIT} de {filteredRecords.length}. Use a busca ou os filtros para chegar ao que procura.</p>}
              {filteredRecords.length === 0 && <p className="muted">Nenhum registro com esses filtros.</p>}
            </>}
          </div>
          <aside className="record-detail">
            {!selectedRecord ? <p className="muted">Selecione um registro para ver todas as linhas em que ele aparece. O estado mostrado é <b>inferido</b> do texto (ex.: “resolvido”, “segue aberto”) e nunca é gravado nos seus arquivos.</p> : <>
              <h3>{selectedRecord.id}{selectedRecord.keyed && <small className="muted"> · {selectedRecord.prefix}</small>}</h3>
              <p>{selectedRecord.status ? <i className={`status-badge ${selectedRecord.status}`}>{selectedRecord.status} (inferido)</i> : <span className="muted">estado não declarado</span>} · {selectedRecord.occurrences} linha(s)</p>
              {recordRows.map((row, i) => <div className="row-block" key={`${row.note}:${row.line}`}>
                <small className="muted">{i === recordRows.length - 1 && recordRows.length > 1 ? "Mais recente · " : ""}{row.note}, linha {row.line}</small>
                <dl>{row.headers.map((header, h) => row.cells[h] ? <div key={h}><dt>{header}</dt><dd>{renderMarkdown(row.cells[h], mdLinks)}</dd></div> : null)}</dl>
                <button type="button" className="secondary-button" onClick={() => { if (byPath.has(row.note)) void open(row.note); }} disabled={!byPath.has(row.note)}>Abrir nota</button>
              </div>)}
            </>}
          </aside>
        </div>}

        {!empty && tab === "graph" && <GraphView notes={notes} edges={index?.edges ?? []} selected={selected} onOpen={(path) => void open(path)} />}

        {!empty && tab === "timeline" && <div className="timeline-view">{timeline.length === 0 ? <p className="notebook-hint">A linha do tempo vem dos recaps dos capítulos. Gere o recap de um capítulo para começar.</p> : timeline.map((entry) => <section key={entry.chapter}><h3>{entry.chapter}</h3><div>{renderMarkdown(entry.events.map((event) => `- ${event}`).join("\n"), { ...mdLinks, onCreate: () => undefined })}</div></section>)}</div>}

        {bibleOpen && <BibleDialog projectRoot={projectRoot} chapters={chapters} activeChapterIndex={activeChapterIndex} ai={ai} genre={genre} glossaryLines={glossaryLines} onClose={() => { setBibleOpen(false); void refresh(); }} onChanged={() => { void refresh(); onChanged(); }} />}
        {dataOpen && <ProjectDataDialog projectRoot={projectRoot} onClose={() => setDataOpen(false)} onChanged={() => { setSelected(""); setContent(""); setSaved(""); setRecordId(""); void refresh(); onChanged(); }} />}
        {recapOpen && activeChapter && <RecapDialog projectRoot={projectRoot} chapterName={activeChapter.name} chapterLines={activeChapterLines} order={activeChapterIndex + 1} ai={ai} genre={genre} glossary={glossaryLines} onClose={() => setRecapOpen(false)} onSaved={() => { void refresh(); onChanged(); }} />}
      </section>
    </div>
  );
}
