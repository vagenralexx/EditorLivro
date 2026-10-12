import { useEffect, useMemo, useState } from "react";
import { emptyGlossary, glossaryDefaults, glossaryLoad, glossarySave, glossarySuggest, type Glossary, type TermCandidate } from "./skillApi";
import "../settings/AiSettingsDialog.css";
import "./GlossaryDialog.css";

interface GlossaryDialogProps {
  projectRoot: string;
  chapterPaths: string[];
  onClose: () => void;
  onSaved: (glossary: Glossary) => void;
}

type Tab = "terms" | "fixed" | "characters" | "profanity";
const split = (text: string) => text.split(/[,;\n]/u).map((item) => item.trim()).filter(Boolean);

export function GlossaryDialog({ projectRoot, chapterPaths, onClose, onSaved }: GlossaryDialogProps) {
  const [glossary, setGlossary] = useState<Glossary>(emptyGlossary);
  const [exists, setExists] = useState(false);
  const [tab, setTab] = useState<Tab>("terms");
  const [candidates, setCandidates] = useState<TermCandidate[] | null>(null);
  const [picked, setPicked] = useState<Set<string>>(new Set());
  const [busy, setBusy] = useState<"" | "loading" | "saving" | "suggesting">("loading");
  const [message, setMessage] = useState<{ kind: "success" | "error"; text: string } | null>(null);
  const [dirty, setDirty] = useState(false);

  useEffect(() => {
    let active = true;
    glossaryLoad(projectRoot).then((state) => { if (!active) return; setGlossary({ ...emptyGlossary(), ...state.glossary }); setExists(state.exists); })
      .catch((error) => active && setMessage({ kind: "error", text: String(error) })).finally(() => active && setBusy(""));
    return () => { active = false; };
  }, [projectRoot]);

  const update = (change: (current: Glossary) => Glossary) => { setGlossary(change); setDirty(true); setMessage(null); };
  const fixedRows = useMemo(() => Object.entries(glossary.traducoes_fixas), [glossary.traducoes_fixas]);

  async function suggest() {
    setBusy("suggesting"); setMessage(null);
    try {
      const found = await glossarySuggest(projectRoot, chapterPaths, [...glossary.nao_traduzir, ...Object.keys(glossary.personagens)]);
      setCandidates(found); setPicked(new Set(found.filter((item) => item.kind === "invented").map((item) => item.term)));
      if (!found.length) setMessage({ kind: "success", text: "Nenhum termo novo encontrado (precisa aparecer ao menos 3 vezes)." });
    } catch (error) { setMessage({ kind: "error", text: String(error) }); } finally { setBusy(""); }
  }

  function addPicked(target: "terms" | "characters") {
    update((current) => target === "terms"
      ? { ...current, nao_traduzir: [...current.nao_traduzir, ...picked] }
      : { ...current, personagens: { ...current.personagens, ...Object.fromEntries([...picked].map((name) => [name, { apelidos: [], tratamentos: [] }])) } });
    setCandidates((current) => current?.filter((item) => !picked.has(item.term)) ?? null);
    setPicked(new Set());
  }

  async function save() {
    setBusy("saving"); setMessage(null);
    try {
      const saved = await glossarySave(projectRoot, glossary);
      setGlossary({ ...emptyGlossary(), ...saved }); setExists(true); setDirty(false);
      onSaved(saved);
      setMessage({ kind: "success", text: "glossario.json salvo na pasta do projeto. A IA passa a respeitar estes termos." });
    } catch (error) { setMessage({ kind: "error", text: String(error) }); } finally { setBusy(""); }
  }

  async function loadDefaults() {
    const defaults = await glossaryDefaults();
    update((current) => ({ ...current, palavroes: { ...defaults, ...current.palavroes } }));
  }

  const tabs: [Tab, string][] = [["terms", `Não traduzir (${glossary.nao_traduzir.length})`], ["fixed", `Traduções fixas (${fixedRows.length})`], ["characters", `Personagens (${Object.keys(glossary.personagens).length})`], ["profanity", `Linguagem crua (${Object.keys(glossary.palavroes).length})`]];
  return (
    <div className="settings-backdrop" role="presentation" onMouseDown={onClose}>
      <section className="settings-dialog glossary-dialog" role="dialog" aria-modal="true" aria-labelledby="glossary-title" onMouseDown={(event) => event.stopPropagation()}>
        <div className="settings-title-row"><div><span className="eyebrow">PROJETO</span><h2 id="glossary-title">Glossário do livro</h2></div><button className="dialog-close" type="button" onClick={onClose} aria-label="Fechar glossário">×</button></div>
        <p className="settings-description">{exists ? "Editando o glossario.json do projeto." : "Ainda não há glossario.json: ele será criado na pasta do projeto ao salvar."} Nomes, termos inventados e traduções fixas são enviados à IA para ela não alterá-los.</p>
        <div className="glossary-tabs" role="tablist">{tabs.map(([id, label]) => <button key={id} type="button" role="tab" aria-selected={tab === id} className={tab === id ? "active" : ""} onClick={() => setTab(id)}>{label}</button>)}</div>

        {tab === "terms" && <div className="glossary-body">
          <label className="settings-field">UM TERMO POR LINHA<textarea rows={6} value={glossary.nao_traduzir.join("\n")} onChange={(event) => update((current) => ({ ...current, nao_traduzir: event.target.value.split("\n") }))} placeholder={"Vhraress\nrwy'n la\nZhrovni"} /></label>
          <button className="secondary-button" type="button" onClick={() => void suggest()} disabled={busy !== "" || !chapterPaths.length}>{busy === "suggesting" ? "Lendo os capítulos…" : "Sugerir termos a partir do livro"}</button>
          {candidates && candidates.length > 0 && <div className="candidate-box"><p>{candidates.length} candidatos (nomes próprios e termos inventados). Marque os que valem:</p>
            <ul>{candidates.map((item) => <li key={item.term}><label><input type="checkbox" checked={picked.has(item.term)} onChange={(event) => setPicked((current) => { const next = new Set(current); if (event.target.checked) next.add(item.term); else next.delete(item.term); return next; })} /> <b>{item.term}</b> <small>{item.count}× em {item.chapters} cap. · {item.kind === "invented" ? "termo inventado" : "nome próprio"}</small></label></li>)}</ul>
            <div className="candidate-actions"><button className="secondary-button" type="button" disabled={!picked.size} onClick={() => addPicked("terms")}>Adicionar em “Não traduzir”</button><button className="secondary-button" type="button" disabled={!picked.size} onClick={() => addPicked("characters")}>Adicionar como personagens</button></div></div>}
        </div>}

        {tab === "fixed" && <div className="glossary-body"><p className="glossary-hint">Expressões que sempre devem ter a mesma tradução (ex.: my heart → meu coração).</p>
          {fixedRows.map(([from, to], index) => <div className="glossary-row" key={index}>
            <input value={from} aria-label="Original" onChange={(event) => update((current) => ({ ...current, traducoes_fixas: Object.fromEntries(Object.entries(current.traducoes_fixas).map(([k, v], i) => i === index ? [event.target.value, v] : [k, v])) }))} />
            <span>→</span>
            <input value={to} aria-label="Tradução" onChange={(event) => update((current) => ({ ...current, traducoes_fixas: { ...current.traducoes_fixas, [from]: event.target.value } }))} />
            <button type="button" className="discard-button" aria-label="Remover" onClick={() => update((current) => { const next = { ...current.traducoes_fixas }; delete next[from]; return { ...current, traducoes_fixas: next }; })}>×</button></div>)}
          <button className="secondary-button" type="button" onClick={() => update((current) => ({ ...current, traducoes_fixas: { ...current.traducoes_fixas, [`termo ${fixedRows.length + 1}`]: "tradução" } }))}>+ Adicionar tradução fixa</button></div>}

        {tab === "characters" && <div className="glossary-body">
          {Object.entries(glossary.personagens).map(([name, info]) => <div className="character-card" key={name}>
            <div className="glossary-row"><strong>{name}</strong><button type="button" className="discard-button" onClick={() => update((current) => { const next = { ...current.personagens }; delete next[name]; return { ...current, personagens: next }; })}>Remover</button></div>
            <label className="settings-field">APELIDOS (separe com vírgula)<input value={info.apelidos.join(", ")} onChange={(event) => update((current) => ({ ...current, personagens: { ...current.personagens, [name]: { ...info, apelidos: split(event.target.value) } } }))} /></label>
            <label className="settings-field">TRATAMENTOS (como é chamado/a pelos outros)<input value={info.tratamentos.join(", ")} onChange={(event) => update((current) => ({ ...current, personagens: { ...current.personagens, [name]: { ...info, tratamentos: split(event.target.value) } } }))} /></label></div>)}
          <NewCharacter onAdd={(name) => update((current) => ({ ...current, personagens: { ...current.personagens, [name]: { apelidos: [], tratamentos: [] } } }))} /></div>}

        {tab === "profanity" && <div className="glossary-body"><p className="glossary-hint">Regra da skill: se o original usa linguagem crua, a tradução usa linguagem crua equivalente. Estas equivalências alimentam a checagem de fidelidade.</p>
          {Object.entries(glossary.palavroes).map(([raw, equivalents]) => <div className="glossary-row" key={raw}>
            <b>{raw}</b><span>→</span>
            <input value={equivalents.join(", ")} aria-label={`Equivalentes de ${raw}`} onChange={(event) => update((current) => ({ ...current, palavroes: { ...current.palavroes, [raw]: split(event.target.value) } }))} />
            <button type="button" className="discard-button" aria-label="Remover" onClick={() => update((current) => { const next = { ...current.palavroes }; delete next[raw]; return { ...current, palavroes: next }; })}>×</button></div>)}
          <div className="candidate-actions"><button className="secondary-button" type="button" onClick={() => void loadDefaults()}>Carregar equivalências da skill</button>
            <NewPair onAdd={(raw) => update((current) => ({ ...current, palavroes: { ...current.palavroes, [raw.toLowerCase()]: [] } }))} /></div></div>}

        {message && <p className={`settings-feedback ${message.kind}`} role={message.kind === "error" ? "alert" : "status"}>{message.text}</p>}
        <div className="settings-actions"><button className="secondary-button" type="button" onClick={onClose}>Fechar</button><button className="primary-button" type="button" onClick={() => void save()} disabled={busy !== "" || (!dirty && exists)}>{busy === "saving" ? "Salvando…" : exists ? "Salvar alterações" : "Criar glossario.json"}</button></div>
      </section>
    </div>
  );
}

function NewCharacter({ onAdd }: { onAdd: (name: string) => void }) {
  const [name, setName] = useState("");
  return <div className="glossary-row"><input value={name} placeholder="Nome do personagem" onChange={(event) => setName(event.target.value)} onKeyDown={(event) => { if (event.key === "Enter" && name.trim()) { onAdd(name.trim()); setName(""); } }} /><button className="secondary-button" type="button" disabled={!name.trim()} onClick={() => { onAdd(name.trim()); setName(""); }}>+ Personagem</button></div>;
}

function NewPair({ onAdd }: { onAdd: (raw: string) => void }) {
  const [raw, setRaw] = useState("");
  return <span className="glossary-row"><input value={raw} placeholder="termo do original (ex.: motherfucker)" onChange={(event) => setRaw(event.target.value)} /><button className="secondary-button" type="button" disabled={!raw.trim()} onClick={() => { onAdd(raw.trim()); setRaw(""); }}>+ Termo</button></span>;
}
