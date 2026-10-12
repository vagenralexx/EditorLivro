import { open } from "@tauri-apps/plugin-dialog";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { useCallback, useEffect, useState } from "react";
import { kindInfo, kindOrder, snapshotCreate, snapshotRestore, snapshotsList, vaultDetectMapping, vaultLocation, vaultSetLocation, type NoteKind, type SnapshotInfo, type VaultLocation } from "./notebookApi";
import "../settings/AiSettingsDialog.css";
import "./Notebook.css";

interface ProjectDataDialogProps {
  projectRoot: string;
  onClose: () => void;
  /** Chamado quando a pasta do caderno mudou ou um backup foi restaurado (o pai recarrega tudo). */
  onChanged: () => void;
}

const sep = (path: string) => (path.includes("\\") ? "\\" : "/");

function formatSize(bytes: number) {
  return bytes > 1024 * 1024 ? `${(bytes / 1024 / 1024).toFixed(1)} MB` : `${Math.max(1, Math.round(bytes / 1024))} KB`;
}

function formatStamp(stamp: string) {
  const match = /^(\d{4})-?(\d{2})-?(\d{2})[T-]?(\d{2}):?(\d{2})/u.exec(stamp);
  return match ? `${match[3]}/${match[2]}/${match[1]} ${match[4]}:${match[5]}` : stamp;
}

export function ProjectDataDialog({ projectRoot, onClose, onChanged }: ProjectDataDialogProps) {
  const [location, setLocation] = useState<VaultLocation | null>(null);
  const [snapshots, setSnapshots] = useState<SnapshotInfo[]>([]);
  const [pending, setPending] = useState<{ path: string; mapping: Record<string, string> } | null>(null);
  const [confirmRestore, setConfirmRestore] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [info, setInfo] = useState("");

  const refresh = useCallback(async () => {
    try { const [where, list] = await Promise.all([vaultLocation(projectRoot), snapshotsList(projectRoot)]); setLocation(where); setSnapshots(list); } catch (reason) { setError(String(reason)); }
  }, [projectRoot]);
  useEffect(() => { void refresh(); }, [refresh]);

  async function reveal(path: string) {
    try { await revealItemInDir(path); } catch (reason) { setError(String(reason)); }
  }

  async function chooseFolder() {
    setError("");
    try {
      const chosen = await open({ directory: true, multiple: false, title: "Escolha a pasta do caderno (ex.: o vault do Obsidian)" });
      if (typeof chosen !== "string") return;
      setPending({ path: chosen, mapping: await vaultDetectMapping(chosen) });
    } catch (reason) { setError(String(reason)); }
  }

  async function applyLocation(path: string | null, mapping: Record<string, string>) {
    setBusy(true); setError(""); setInfo("");
    try {
      const next = await vaultSetLocation(projectRoot, path, mapping);
      setLocation(next); setPending(null); onChanged();
      setInfo(path ? "Pasta do caderno atualizada. Nenhuma nota foi movida ou alterada." : "Voltou à pasta padrão do projeto.");
    } catch (reason) { setError(String(reason)); } finally { setBusy(false); }
  }

  async function createBackup() {
    setBusy(true); setError(""); setInfo("");
    try { await snapshotCreate(projectRoot, "manual"); await refresh(); setInfo("Cópia de segurança criada."); } catch (reason) { setError(String(reason)); } finally { setBusy(false); }
  }

  async function restore(name: string) {
    setBusy(true); setError(""); setInfo("");
    try {
      const result = await snapshotRestore(projectRoot, name);
      setConfirmRestore(""); await refresh(); onChanged();
      setInfo(`${result.restored} arquivo(s) restaurado(s). O estado anterior foi guardado em ${result.safetySnapshot}.`);
    } catch (reason) { setError(String(reason)); } finally { setBusy(false); }
  }

  const s = sep(projectRoot);
  const files = [
    { label: "Configurações, posição de leitura, EPUB e original", path: `${projectRoot}${s}.livro${s}projeto.json` },
    { label: "Glossário", path: `${projectRoot}${s}glossario.json` },
    { label: "Caderno / bíblia (notas .md)", path: location?.path ?? "" },
    { label: "Cópias de segurança", path: `${projectRoot}${s}.livro${s}snapshots` },
  ];

  return (
    <div className="settings-backdrop" role="presentation" onMouseDown={onClose}>
      <section className="settings-dialog project-data" role="dialog" aria-modal="true" aria-labelledby="project-data-title" onMouseDown={(event) => event.stopPropagation()}>
        <div className="settings-title-row"><div><span className="eyebrow">SEGURANÇA DOS DADOS</span><h2 id="project-data-title">Dados do projeto</h2></div><button className="dialog-close" type="button" onClick={onClose} aria-label="Fechar">×</button></div>
        <p className="settings-description">Tudo que o app sabe sobre este livro fica <b>dentro da pasta do projeto</b>, em arquivos abertos (JSON e Markdown). Reinstalar o app ou trocar de computador não perde nada: basta abrir a mesma pasta.</p>
        {error && <p className="settings-feedback error" role="alert">{error}</p>}
        {info && <p className="settings-feedback success" role="status">{info}</p>}

        <h3 className="pd-heading">Onde está cada coisa</h3>
        <ul className="pd-files">{files.map((file) => <li key={file.label}><div><b>{file.label}</b><code>{file.path || "…"}</code></div><button className="secondary-button" type="button" disabled={!file.path} onClick={() => void reveal(file.path)}>Mostrar</button></li>)}</ul>

        <h3 className="pd-heading">Pasta do caderno</h3>
        <p className="settings-description pd-tight">Use a pasta que você já tem (por exemplo o vault do Obsidian da trilogia) — as notas continuam suas, o app só lê e escreve nelas quando você pede.</p>
        {!pending && <div className="pd-row"><code>{location?.path ?? "…"}</code><span className="muted">{location?.isDefault ? "padrão do projeto" : location?.exists ? "pasta escolhida" : "pasta não encontrada"}</span></div>}
        {!pending && <div className="settings-actions pd-actions"><button className="secondary-button" type="button" onClick={() => void chooseFolder()} disabled={busy}>Escolher outra pasta…</button>{!location?.isDefault && <button className="secondary-button" type="button" onClick={() => void applyLocation(null, {})} disabled={busy}>Voltar ao padrão</button>}</div>}
        {pending && <div className="pd-mapping">
          <p><b>{pending.path}</b></p>
          <p className="muted">O app reconheceu estas pastas pelo nome. Ajuste o que cada uma guarda; pastas não listadas continuam sendo lidas como notas comuns.</p>
          {Object.keys(pending.mapping).length === 0 ? <p className="muted">Nenhuma pasta reconhecida. Notas novas serão criadas nas pastas padrão do app.</p> : <table><thead><tr><th>Pasta</th><th>Guarda</th></tr></thead><tbody>{Object.entries(pending.mapping).sort(([a], [b]) => a.localeCompare(b, "pt-BR")).map(([folder, kind]) => <tr key={folder}><td><code>{folder}</code></td><td><select value={kind} onChange={(event) => setPending({ ...pending, mapping: { ...pending.mapping, [folder]: event.target.value } })}>{kindOrder.map((id: NoteKind) => <option key={id} value={id}>{kindInfo[id].label}</option>)}</select></td></tr>)}</tbody></table>}
          <div className="settings-actions"><button className="secondary-button" type="button" onClick={() => setPending(null)} disabled={busy}>Cancelar</button><button className="primary-button" type="button" onClick={() => void applyLocation(pending.path, pending.mapping)} disabled={busy}>Usar esta pasta</button></div>
        </div>}

        <h3 className="pd-heading">Cópias de segurança</h3>
        <p className="settings-description pd-tight">Feitas sozinhas antes de renomear, excluir ou aplicar recaps (no máximo uma a cada 5 min; guardamos as 30 mais recentes). Restaurar nunca apaga o estado atual: ele vira outra cópia.</p>
        <div className="settings-actions pd-actions"><button className="primary-button" type="button" onClick={() => void createBackup()} disabled={busy}>Criar cópia agora</button></div>
        {snapshots.length === 0 ? <p className="muted">Nenhuma cópia ainda.</p> : <ul className="pd-snapshots">{snapshots.map((snap) => <li key={snap.name}><div><b>{formatStamp(snap.createdAt)}</b> <span className="muted">{snap.reason || "—"} · {formatSize(snap.sizeBytes)}</span></div>
          {confirmRestore === snap.name ? <div className="pd-confirm"><span>Substituir os arquivos atuais?</span><button className="primary-button" type="button" onClick={() => void restore(snap.name)} disabled={busy}>Restaurar</button><button className="secondary-button" type="button" onClick={() => setConfirmRestore("")}>Cancelar</button></div> : <button className="secondary-button" type="button" onClick={() => setConfirmRestore(snap.name)} disabled={busy}>Restaurar…</button>}</li>)}</ul>}
      </section>
    </div>
  );
}
