import { useCallback, useEffect, useRef, useState, type PointerEvent as ReactPointerEvent, type ReactNode } from "react";
import "./Assistant.css";

export interface ResultFinding { title: string; severity: string; detail: string; anchor: string; }
export interface ResultSuggestion { before: string; after: string; reason: string; }
export type ResultsTab = "resultado" | "sugestoes" | "diagnostico";

interface ResultsPanelProps {
  tab: ResultsTab;
  onTab: (tab: ResultsTab) => void;
  onClose: () => void;
  summary: string;
  error: string;
  findings: ResultFinding[];
  suggestions: ResultSuggestion[];
  onApply: (suggestion: ResultSuggestion) => void;
  /** Nota de memória, Genesis Score e comparação com o original (aparecem em "Resultado"). */
  extra: ReactNode;
  diagnostics: ReactNode;
}

const WIDTH = 440;

/**
 * Painel flutuante com o resultado da análise: pode ser arrastado pelo cabeçalho, redimensionado
 * pelo canto e recolhido, para não tapar o texto que está sendo revisado.
 */
export function ResultsPanel({ tab, onTab, onClose, summary, error, findings, suggestions, onApply, extra, diagnostics }: ResultsPanelProps) {
  const [position, setPosition] = useState(() => ({ left: Math.max(12, window.innerWidth - 340 - WIDTH - 20), top: 78 }));
  const [collapsed, setCollapsed] = useState(false);
  const drag = useRef<{ x: number; y: number; left: number; top: number } | null>(null);
  const panelRef = useRef<HTMLElement>(null);

  const clamp = useCallback((left: number, top: number) => ({ left: Math.min(Math.max(8, left), Math.max(8, window.innerWidth - 120)), top: Math.min(Math.max(8, top), Math.max(8, window.innerHeight - 56)) }), []);
  useEffect(() => {
    const onResize = () => setPosition((current) => clamp(current.left, current.top));
    window.addEventListener("resize", onResize);
    return () => window.removeEventListener("resize", onResize);
  }, [clamp]);

  const onPointerDown = (event: ReactPointerEvent<HTMLElement>) => {
    if ((event.target as Element).closest("button")) return;
    drag.current = { x: event.clientX, y: event.clientY, left: position.left, top: position.top };
    event.currentTarget.setPointerCapture(event.pointerId);
  };
  const onPointerMove = (event: ReactPointerEvent<HTMLElement>) => {
    const start = drag.current;
    if (start) setPosition(clamp(start.left + event.clientX - start.x, start.top + event.clientY - start.y));
  };
  const endDrag = () => { drag.current = null; };

  const tabs: [ResultsTab, string][] = [["resultado", findings.length ? `Resultado (${findings.length})` : "Resultado"], ["sugestoes", `Sugestões (${suggestions.length})`], ["diagnostico", "Diagnóstico"]];

  return (
    <section ref={panelRef} className={`results-panel ${collapsed ? "collapsed" : ""}`} style={{ left: position.left, top: position.top }} role="region" aria-label="Resultados da análise" onKeyDown={(event) => { if (event.key === "Escape") { event.stopPropagation(); onClose(); } }}>
      <header className="results-head" onPointerDown={onPointerDown} onPointerMove={onPointerMove} onPointerUp={endDrag} onPointerCancel={endDrag}>
        <h2>{error ? "Não foi possível analisar" : "Resultados"}</h2>
        <span className="results-grip" aria-hidden="true">⠿</span>
        <button type="button" className="icon-button" onClick={() => setCollapsed((value) => !value)} aria-label={collapsed ? "Expandir resultados" : "Recolher resultados"} aria-expanded={!collapsed}>{collapsed ? "▢" : "–"}</button>
        <button type="button" className="icon-button" onClick={onClose} aria-label="Fechar resultados">×</button>
      </header>
      {!collapsed && <>
        <div className="results-tabs" role="tablist" aria-label="Seções dos resultados">
          {tabs.map(([id, label]) => <button key={id} type="button" role="tab" id={`results-tab-${id}`} aria-selected={tab === id} aria-controls={`results-body-${id}`} className={tab === id ? "active" : ""} onClick={() => onTab(id)}>{label}</button>)}
        </div>
        <div className="results-body" role="tabpanel" id={`results-body-${tab}`} aria-labelledby={`results-tab-${tab}`} tabIndex={0}>
          {tab === "resultado" && (error ? <p className="analysis-result error">{error}</p> : <>
            {summary && <p className="analysis-summary">{summary}</p>}
            {findings.length === 0 && !summary && <p className="assistant-help">Nenhum achado nesta análise.</p>}
            {findings.map((finding, index) => <article className="finding-card" key={`${finding.title}-${index}`}><header><strong>{finding.title}</strong><span className={`severity ${finding.severity.toLocaleLowerCase("pt-BR")}`}>{finding.severity}</span></header><p>{finding.detail}</p>{finding.anchor && <blockquote><strong>Trecho:</strong> “{finding.anchor}”</blockquote>}</article>)}
            {extra}
          </>)}
          {tab === "sugestoes" && (suggestions.length === 0 ? <p className="assistant-help">Nenhuma sugestão aplicável. Rode uma análise com perfil de revisão para receber trechos ANTES/DEPOIS.</p> : suggestions.map((suggestion, index) => (
            <article className="suggestion-card" key={`${suggestion.before}-${index}`}>
              <div><strong>Antes</strong><del>{suggestion.before}</del></div>
              <div><strong>Depois</strong><ins>{suggestion.after}</ins></div>
              <small>{suggestion.reason}</small>
              <button type="button" onClick={() => onApply(suggestion)}>Aplicar sugestão</button>
            </article>)))}
          {tab === "diagnostico" && diagnostics}
        </div>
      </>}
    </section>
  );
}
