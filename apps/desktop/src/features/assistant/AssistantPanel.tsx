import { useId, useState, type ReactNode } from "react";
import { useDialogA11y } from "../../shared/useDialogA11y";
import "./Assistant.css";

export interface AssistantProfile { id: string; label: string; description: string; tier: string; usesMemory: boolean; disabled: boolean; }

interface AssistantPanelProps {
  /** Resumo do modelo escolhido para o botão do topo do painel. */
  model: { badge: string; provider: string; name: string | null; effort: string | null; ready: boolean };
  /** Conteúdo do seletor de modelo (servidor, busca, modelo, esforço, chave). */
  modelPicker: ReactNode;
  profiles: AssistantProfile[];
  profileId: string;
  onProfile: (id: string) => void;
  scope: "page" | "chapter";
  onScope: (scope: "page" | "chapter") => void;
  pageLabel: string;
  onAnalyze: () => void;
  analyzing: boolean;
  progressText: string;
  onCancel: () => void;
  /** Por que não dá para analisar agora (mostrado sob o botão); `null` = pode analisar. */
  blocker: string | null;
  results: { findings: number; suggestions: number; error: boolean; hasAny: boolean };
  onOpenResults: () => void;
  onOpenDiagnostics: () => void;
  optionsSummary: string;
  options: ReactNode;
}

function ModelPopover({ onClose, children }: { onClose: () => void; children: ReactNode }) {
  const ref = useDialogA11y<HTMLDivElement>(onClose);
  return (
    <>
      <div className="ai-popover-scrim" role="presentation" onMouseDown={onClose} />
      <div ref={ref} tabIndex={-1} className="ai-popover" role="dialog" aria-label="Modelo de IA">
        <div className="ai-popover-head"><strong>Modelo de IA</strong><button type="button" className="dialog-close" onClick={onClose} aria-label="Fechar seletor de modelo">×</button></div>
        {children}
      </div>
    </>
  );
}

export function AssistantPanel({ model, modelPicker, profiles, profileId, onProfile, scope, onScope, pageLabel, onAnalyze, analyzing, progressText, onCancel, blocker, results, onOpenResults, onOpenDiagnostics, optionsSummary, options }: AssistantPanelProps) {
  const [pickerOpen, setPickerOpen] = useState(false);
  const hintId = useId();
  const profile = profiles.find((item) => item.id === profileId) ?? profiles[0];
  const resultsLabel = results.error ? "Ver o erro da análise" : `Ver resultados · ${results.findings} achado${results.findings === 1 ? "" : "s"} · ${results.suggestions} sugest${results.suggestions === 1 ? "ão" : "ões"}`;

  return (
    <aside className="ai-panel assistant" aria-label="Assistente de IA">
      <div className="assistant-model">
        <span className="assistant-eyebrow">MODELO</span>
        <button type="button" className={`model-chip ${model.ready ? "" : "warn"}`} onClick={() => setPickerOpen(true)} aria-haspopup="dialog" aria-expanded={pickerOpen} title="Trocar modelo, servidor ou esforço de raciocínio">
          <span className="model-badge">{model.badge}</span>
          <span className="model-text"><strong>{model.name ?? "Escolher um modelo"}</strong><small>{model.provider}{model.effort ? ` · esforço ${model.effort}` : ""}</small></span>
          <span aria-hidden="true" className="chevron">▾</span>
        </button>
        {pickerOpen && <ModelPopover onClose={() => setPickerOpen(false)}>{modelPicker}</ModelPopover>}
      </div>

      <section className="assistant-card" aria-labelledby="assistant-run-title">
        <h2 id="assistant-run-title">Analisar com IA</h2>
        <label className="assistant-field">Perfil editorial
          <select value={profile?.id ?? ""} onChange={(event) => onProfile(event.target.value)} disabled={profiles.length === 0}>
            {profiles.map((item) => <option key={item.id} value={item.id} disabled={item.disabled}>{item.label}{item.disabled ? " (carregue o original)" : ""}</option>)}
          </select>
        </label>
        {profile && <p className="assistant-help">{profile.description}{profile.usesMemory ? " Usa a memória do caderno." : ""}</p>}
        <fieldset className="assistant-scope">
          <legend>O que analisar</legend>
          <label><input type="radio" name="analysis-scope" checked={scope === "page"} onChange={() => onScope("page")} /><span>{pageLabel}</span></label>
          <label><input type="radio" name="analysis-scope" checked={scope === "chapter"} onChange={() => onScope("chapter")} /><span>Capítulo inteiro</span></label>
        </fieldset>
        <button className="primary-button wide-button assistant-run" type="button" onClick={onAnalyze} disabled={analyzing || blocker !== null} aria-describedby={blocker ? hintId : undefined}>{analyzing ? "Analisando…" : scope === "chapter" ? "Analisar capítulo" : `Analisar ${pageLabel.toLowerCase()}`}</button>
        {blocker && !analyzing && <p id={hintId} className="assistant-blocker" role="status">{blocker}</p>}
        {analyzing && <div className="analysis-progress" role="status" aria-live="polite"><span>{progressText}</span><button type="button" className="discard-button" onClick={onCancel}>Cancelar</button></div>}
        {results.hasAny && !analyzing && <button type="button" className={`assistant-results-link ${results.error ? "error" : ""}`} onClick={onOpenResults}>{resultsLabel}</button>}
      </section>

      <section className="assistant-card" aria-labelledby="assistant-tools-title">
        <h2 id="assistant-tools-title">Sem IA</h2>
        <p className="assistant-help">Diagnóstico instantâneo do capítulo: ritmo, diálogo, padrões de texto de IA e sugestões de fluidez.</p>
        <button type="button" className="secondary-button wide-button" onClick={onOpenDiagnostics}>Abrir diagnóstico</button>
      </section>

      <details className="assistant-options">
        <summary><span>Opções deste livro</span><small>{optionsSummary}</small></summary>
        <div className="assistant-options-body">{options}</div>
      </details>

      <p className="assistant-privacy">Só o trecho analisado é enviado ao provedor. A chave fica no Gerenciador de Credenciais do Windows, nunca no XHTML ou no EPUB.</p>
    </aside>
  );
}
