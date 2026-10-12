import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";
import { friendlyModelName, providerFor, providers, type ProviderId } from "../ai/providers";
import "./AiSettingsDialog.css";

interface ConnectionResult {
  provider: string;
  message: string;
  models: string[];
}

interface AiSettingsDialogProps {
  activeProviderId: ProviderId;
  models: Record<ProviderId, string>;
  availableModels: Record<ProviderId, string[]>;
  onClose: () => void;
  onProviderChange: (provider: ProviderId) => void;
  onModelChange: (provider: ProviderId, model: string) => void;
  onModelsLoaded: (provider: ProviderId, models: string[]) => void;
  onKeyStatusChange: (provider: ProviderId, saved: boolean) => void;
}

export function AiSettingsDialog({ activeProviderId, models, availableModels, onClose, onProviderChange, onModelChange, onModelsLoaded, onKeyStatusChange }: AiSettingsDialogProps) {
  const [apiKey, setApiKey] = useState("");
  const [hasSavedKey, setHasSavedKey] = useState(false);
  const [isChecking, setIsChecking] = useState(true);
  const [isSaving, setIsSaving] = useState(false);
  const [isTesting, setIsTesting] = useState(false);
  const [feedback, setFeedback] = useState<{ kind: "success" | "error"; text: string } | null>(null);
  const provider = providerFor(activeProviderId);
  const model = models[activeProviderId];
  const providerModels = [...availableModels[activeProviderId]].sort((left, right) => friendlyModelName(left).localeCompare(friendlyModelName(right), "pt-BR"));

  useEffect(() => {
    let current = true;
    setApiKey("");
    setFeedback(null);
    setIsChecking(true);
    invoke<boolean>("has_provider_api_key", { provider: activeProviderId })
      .then((saved) => {
        if (!current) return;
        setHasSavedKey(saved);
        onKeyStatusChange(activeProviderId, saved);
      })
      .catch((error) => current && setFeedback({ kind: "error", text: String(error) }))
      .finally(() => current && setIsChecking(false));
    return () => { current = false; };
  }, [activeProviderId, onKeyStatusChange]);

  async function saveKey() {
    if (!apiKey.trim()) {
      setFeedback({ kind: "error", text: "Cole uma chave antes de salvar." });
      return false;
    }
    setIsSaving(true);
    setFeedback(null);
    try {
      await invoke("save_provider_api_key", { provider: activeProviderId, apiKey });
      setApiKey("");
      setHasSavedKey(true);
      onKeyStatusChange(activeProviderId, true);
      // A nova chave pode pertencer a outra conta, portanto o catálogo anterior
      // só volta a ser usado depois de uma validação explícita.
      onModelsLoaded(activeProviderId, []);
      onModelChange(activeProviderId, "");
      setFeedback({ kind: "success", text: "Chave salva no cofre de credenciais do Windows." });
      return true;
    } catch (error) {
      setFeedback({ kind: "error", text: String(error) });
      return false;
    } finally {
      setIsSaving(false);
    }
  }

  async function testConnection() {
    if (apiKey.trim() && !(await saveKey())) return;
    if (!apiKey.trim() && !hasSavedKey) {
      setFeedback({ kind: "error", text: "Salve uma chave antes de testar a conexão." });
      return;
    }
    setIsTesting(true);
    setFeedback(null);
    try {
      const result = await invoke<ConnectionResult>("test_ai_provider", { input: { provider: activeProviderId } });
      onModelsLoaded(activeProviderId, result.models);
      if (!model || !result.models.includes(model)) onModelChange(activeProviderId, result.models[0] ?? "");
      setFeedback({ kind: "success", text: result.message });
    } catch (error) {
      setFeedback({ kind: "error", text: String(error) });
    } finally {
      setIsTesting(false);
    }
  }

  return (
    <div className="settings-backdrop" role="presentation" onMouseDown={onClose}>
      <section className="settings-dialog" role="dialog" aria-modal="true" aria-labelledby="settings-title" onMouseDown={(event) => event.stopPropagation()}>
        <div className="settings-title-row"><div><span className="eyebrow">CONFIGURAÇÕES</span><h2 id="settings-title">Provedor e credenciais de IA</h2></div><button className="dialog-close" type="button" onClick={onClose} aria-label="Fechar configurações">×</button></div>
        <p className="settings-description">A chave não é gravada no projeto, no EPUB nem em arquivos de texto. Ela fica apenas no Gerenciador de Credenciais do Windows.</p>
        <label className="settings-field">PROVEDOR
          <select value={activeProviderId} onChange={(event) => onProviderChange(event.target.value as ProviderId)}>
            {providers.map((item) => <option key={item.id} value={item.id}>{item.label}</option>)}
          </select>
        </label>
        <label className="settings-field">MODELO
          <select value={model} onChange={(event) => onModelChange(activeProviderId, event.target.value)} disabled={!providerModels.length}>
            <option value="">{providerModels.length ? "Selecione um modelo" : "Valide a chave para carregar os modelos"}</option>
            {providerModels.map((availableModel) => <option key={availableModel} value={availableModel}>{friendlyModelName(availableModel)} · {availableModel}</option>)}
          </select>
        </label>
        <div className="endpoint-card"><span>{provider.badge}</span><div><strong>{provider.label}</strong><small>{provider.endpoint}</small></div></div>
        <label className="settings-field">CHAVE DE API
          <input type="password" value={apiKey} onChange={(event) => setApiKey(event.target.value)} placeholder={isChecking ? "Verificando cofre…" : hasSavedKey ? "Chave já salva — cole outra para substituir" : "Cole a chave deste provedor"} autoComplete="new-password" />
        </label>
        <p className="key-status">{isChecking ? "Verificando cofre…" : hasSavedKey ? providerModels.length ? `✓ Configuração restaurada; ${providerModels.length} modelos carregados.` : "✓ Há uma chave salva. Use “Testar conexão” para validá-la e carregar os modelos." : "Nenhuma chave salva para este provedor."}</p>
        {feedback && <p className={`settings-feedback ${feedback.kind}`}>{feedback.text}</p>}
        <div className="settings-actions"><button className="secondary-button" type="button" onClick={saveKey} disabled={isSaving || isTesting}>{isSaving ? "Salvando…" : "Salvar chave"}</button><button className="primary-button" type="button" onClick={testConnection} disabled={isSaving || isTesting || isChecking}>{isTesting ? "Testando…" : "Testar conexão"}</button></div>
      </section>
    </div>
  );
}
