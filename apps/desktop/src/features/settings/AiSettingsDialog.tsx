import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useRef, useState, type FormEvent } from "react";
import { detectProviderFromKey, friendlyModelName, providerFor, providers, type ProviderId } from "../ai/providers";
import "./AiSettingsDialog.css";
import { useDialogA11y } from "../../shared/useDialogA11y";

interface ConnectionResult {
  provider: string;
  message: string;
  models: string[];
}

interface AiSettingsDialogProps {
  activeProviderId: ProviderId;
  models: Record<ProviderId, string>;
  availableModels: Record<ProviderId, string[]>;
  keyStatus: Partial<Record<ProviderId, boolean>>;
  onClose: () => void;
  onProviderChange: (provider: ProviderId) => void;
  onModelChange: (provider: ProviderId, model: string) => void;
  onModelsLoaded: (provider: ProviderId, models: string[]) => void;
  onKeyStatusChange: (provider: ProviderId, saved: boolean) => void;
}

export function AiSettingsDialog({ activeProviderId, models, availableModels, keyStatus, onClose, onProviderChange, onModelChange, onModelsLoaded, onKeyStatusChange }: AiSettingsDialogProps) {
  const [apiKey, setApiKey] = useState("");
  const [hasSavedKey, setHasSavedKey] = useState(false);
  const [isChecking, setIsChecking] = useState(true);
  const [isConnecting, setIsConnecting] = useState(false);
  const [isRemoving, setIsRemoving] = useState(false);
  const [confirmRemove, setConfirmRemove] = useState(false);
  const keyToKeepRef = useRef<string | null>(null);
  const [detectedNotice, setDetectedNotice] = useState("");
  const [feedback, setFeedback] = useState<{ kind: "success" | "error"; text: string } | null>(null);
  const provider = providerFor(activeProviderId);
  const model = models[activeProviderId];
  const providerModels = [...availableModels[activeProviderId]].sort((left, right) => friendlyModelName(left).localeCompare(friendlyModelName(right), "pt-BR"));
  const busy = isConnecting || isRemoving;
  const hasCatalog = availableModels[activeProviderId].length > 0;

  // Conecta (valida + grava, se houver chave nova) e carrega o catálogo de uma só vez.
  const connect = useCallback(async (providerId: ProviderId, newKey: string | null, currentModel: string) => {
    setIsConnecting(true);
    setFeedback(null);
    try {
      const result = await invoke<ConnectionResult>("connect_ai_provider", { provider: providerId, apiKey: newKey });
      setApiKey("");
      setHasSavedKey(true);
      onKeyStatusChange(providerId, true);
      onModelsLoaded(providerId, result.models);
      if (!currentModel || !result.models.includes(currentModel)) onModelChange(providerId, result.models[0] ?? "");
      setFeedback({ kind: "success", text: result.message });
    } catch (error) {
      setFeedback({ kind: "error", text: String(error) });
    } finally {
      setIsConnecting(false);
    }
  }, [onKeyStatusChange, onModelChange, onModelsLoaded]);

  useEffect(() => {
    let current = true;
    // Ao trocar de servidor por detecção da chave colada, a chave digitada é mantida.
    setApiKey(keyToKeepRef.current ?? "");
    keyToKeepRef.current = null;
    setFeedback(null);
    setConfirmRemove(false);
    setIsChecking(true);
    invoke<boolean>("has_provider_api_key", { provider: activeProviderId })
      .then((saved) => {
        if (!current) return;
        setHasSavedKey(saved);
        onKeyStatusChange(activeProviderId, saved);
        // Chave salva mas catálogo vazio (primeira abertura ou cache limpo): carrega sozinho.
        if (saved && !availableModels[activeProviderId].length) void connect(activeProviderId, null, models[activeProviderId]);
      })
      .catch((error) => current && setFeedback({ kind: "error", text: String(error) }))
      .finally(() => current && setIsChecking(false));
    return () => { current = false; };
    // Só reexecuta ao trocar de provedor; catálogo e modelo são lidos no momento da troca.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [activeProviderId]);

  function handleKeyInput(value: string) {
    setApiKey(value);
    const detected = detectProviderFromKey(value);
    if (detected && detected !== activeProviderId) {
      keyToKeepRef.current = value;
      setDetectedNotice(`Chave reconhecida como ${providerFor(detected).label}; o servidor foi ajustado.`);
      onProviderChange(detected);
    } else if (!value.trim()) {
      setDetectedNotice("");
    }
  }

  function submit(event?: FormEvent) {
    event?.preventDefault();
    if (busy) return;
    if (!apiKey.trim() && !hasSavedKey) {
      setFeedback({ kind: "error", text: "Cole a chave de API deste provedor para continuar." });
      return;
    }
    void connect(activeProviderId, apiKey.trim() || null, model);
  }

  async function removeKey() {
    setIsRemoving(true);
    setFeedback(null);
    try {
      await invoke("delete_provider_api_key", { provider: activeProviderId });
      setHasSavedKey(false);
      setConfirmRemove(false);
      setApiKey("");
      onKeyStatusChange(activeProviderId, false);
      onModelsLoaded(activeProviderId, []);
      onModelChange(activeProviderId, "");
      setFeedback({ kind: "success", text: "Chave removida do cofre de credenciais do Windows." });
    } catch (error) {
      setFeedback({ kind: "error", text: String(error) });
    } finally {
      setIsRemoving(false);
    }
  }

  const dialogRef = useDialogA11y<HTMLElement>(onClose);
  return (
    <div className="settings-backdrop" role="presentation" onMouseDown={onClose}>
      <section className="settings-dialog" ref={dialogRef} tabIndex={-1} role="dialog" aria-modal="true" aria-labelledby="settings-title" onMouseDown={(event) => event.stopPropagation()}><form className="settings-form" onSubmit={submit}>
        <div className="settings-title-row"><div><span className="eyebrow">CONFIGURAÇÕES</span><h2 id="settings-title">Provedor e credenciais de IA</h2></div><button className="dialog-close" type="button" onClick={onClose} aria-label="Fechar configurações">×</button></div>
        <p className="settings-description">A chave não é gravada no projeto, no EPUB nem em arquivos de texto. Ela fica apenas no Gerenciador de Credenciais do Windows.</p>
        <label className="settings-field">SERVIDOR (PROVEDOR)
          <select value={activeProviderId} onChange={(event) => onProviderChange(event.target.value as ProviderId)}>
            {providers.map((item) => <option key={item.id} value={item.id}>{keyStatus[item.id] ? "✓ " : ""}{item.label}</option>)}
          </select>
        </label>
        <label className="settings-field">MODELO
          <select value={model} onChange={(event) => onModelChange(activeProviderId, event.target.value)} disabled={!providerModels.length || busy}>
            <option value="">{providerModels.length ? "Selecione um modelo" : isConnecting ? "Carregando modelos…" : "Conecte a chave para carregar os modelos"}</option>
            {providerModels.map((availableModel) => <option key={availableModel} value={availableModel}>{friendlyModelName(availableModel)} · {availableModel}</option>)}
          </select>
        </label>
        <div className="endpoint-card"><span>{provider.badge}</span><div><strong>{provider.label}</strong><small>{provider.endpoint}</small></div></div>
        <label className="settings-field">CHAVE DE API
          <input type="password" value={apiKey} onChange={(event) => handleKeyInput(event.target.value)} placeholder={isChecking ? "Verificando cofre…" : hasSavedKey ? "Chave já salva — cole outra para substituir" : "Cole a chave deste provedor"} disabled={busy} autoComplete="new-password" />
        </label>
        {detectedNotice && <p className="settings-feedback success" role="status">{detectedNotice}</p>}
        <p className="key-status key-hint">{provider.keyHint}</p>
        <p className="key-status">{isChecking ? "Verificando cofre…" : isConnecting ? "Validando a chave e carregando modelos…" : hasSavedKey ? hasCatalog ? `✓ Chave salva · ${providerModels.length} modelos disponíveis.` : "✓ Há uma chave salva. Use “Atualizar modelos”." : "Nenhuma chave salva para este provedor."}</p>
        {feedback && <p className={`settings-feedback ${feedback.kind}`} role={feedback.kind === "error" ? "alert" : "status"}>{feedback.text}</p>}
        <div className="settings-actions">
          {hasSavedKey && !confirmRemove && <button className="danger-button" type="button" onClick={() => setConfirmRemove(true)} disabled={busy}>Remover chave</button>}
          {confirmRemove && <><span className="remove-confirm">Remover a chave de {provider.label}?</span><button className="secondary-button" type="button" onClick={() => setConfirmRemove(false)} disabled={isRemoving}>Cancelar</button><button className="danger-button strong" type="button" onClick={() => void removeKey()} disabled={isRemoving}>{isRemoving ? "Removendo…" : "Sim, remover"}</button></>}
          {!confirmRemove && <button className="primary-button" type="submit" disabled={busy || isChecking}>{isConnecting ? "Conectando…" : apiKey.trim() ? "Conectar e carregar modelos" : hasSavedKey ? "Atualizar modelos" : "Conectar e carregar modelos"}</button>}
        </div>
      </form></section>
    </div>
  );
}
