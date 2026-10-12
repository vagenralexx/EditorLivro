import { useEffect, useState } from "react";
import { scanEditorial, type FluencySuggestion, type ScanOptions, type ScanReport } from "./skillApi";
import "./SkillPanels.css";

interface ScanPanelProps {
  /** Texto do capítulo, uma linha por parágrafo. */
  text: string;
  options: ScanOptions;
  genreLabel: string;
  onApplyFluency: (suggestion: FluencySuggestion) => void;
}

const kindLabel: Record<string, string> = { dialogo: "Diálogo", ritmo: "Ritmo", hook: "Abertura", suavizacao: "Suavização?", "anti-ia": "Anti-IA" };

export function ScanPanel({ text, options, genreLabel, onApplyFluency }: ScanPanelProps) {
  const [report, setReport] = useState<ScanReport | null>(null);

  // Recalcula sem travar a digitação: atraso curto e descarte de respostas antigas.
  useEffect(() => {
    if (!text.trim()) { setReport(null); return undefined; }
    let active = true;
    const timer = window.setTimeout(() => { scanEditorial(text, options).then((result) => active && setReport(result)).catch(() => active && setReport(null)); }, 450);
    return () => { active = false; window.clearTimeout(timer); };
  }, [text, options.genre, options.translationMode, options.adultContent, options.isOpening]);

  if (!report) return <section className="scan-panel"><strong>Diagnóstico instantâneo</strong><p className="scan-muted">Abra um capítulo para ver as métricas editoriais (sem IA, sem custo).</p></section>;
  const target = report.dialogueTarget;
  const dialogueOk = !target || (report.dialoguePercent >= target[0] && report.dialoguePercent <= target[1]);
  const patternsOk = report.patternsPer3k <= report.patternsMax;
  return (
    <section className="scan-panel" aria-label="Diagnóstico instantâneo do capítulo">
      <div className="scan-title"><strong>Diagnóstico instantâneo</strong><span>sem IA · {genreLabel}</span></div>
      <div className="scan-grid">
        <div><b>{report.words.toLocaleString("pt-BR")}</b><small>palavras</small></div>
        <div><b>{report.avgSentenceWords.toFixed(1)}</b><small>palavras/frase</small></div>
        <div className={dialogueOk ? "" : "warn"}><b>{report.dialoguePercent.toFixed(0)}%</b><small>diálogo{target ? ` (alvo ${target[0]}–${target[1]}%)` : ""}</small></div>
        <div className={patternsOk ? "" : "warn"}><b>{report.patternTotal}</b><small>padrões anti-IA · {report.patternsPer3k.toFixed(1)}/3 mil (meta ≤ {report.patternsMax})</small></div>
      </div>
      {report.patterns.length > 0 && <details className="scan-details"><summary>Padrões encontrados ({report.patterns.length} tipo{report.patterns.length === 1 ? "" : "s"})</summary>
        <ul>{report.patterns.map((pattern) => <li key={pattern.id}><b>#{pattern.id} {pattern.name}</b> — {pattern.count}×{pattern.examples.map((example, index) => <q key={index}>{example}</q>)}</li>)}</ul>
        <p className="scan-muted">São candidatos por regra: confirme na leitura. A varredura cobre {report.patternsCovered} dos 20 padrões; os demais exigem a análise “Varredura anti-IA”.</p></details>}
      {report.notes.length > 0 && <ul className="scan-notes">{report.notes.map((note, index) => <li key={index}><span>{kindLabel[note.kind] ?? note.kind}</span>{note.message}{note.example && <q>{note.example}</q>}</li>)}</ul>}
      {report.tags.length > 0 && <p className="scan-muted">Tags de fala: {report.tags.slice(0, 4).map((tag) => `${tag.tag} ${tag.count}`).join(" · ")}</p>}
      {report.fluency.length > 0 && <details className="scan-details" open><summary>Fluidez PT-BR ({report.fluency.length} sugest{report.fluency.length === 1 ? "ão" : "ões"} da skill)</summary>
        <ul className="fluency-list">{report.fluency.map((suggestion) => <li key={suggestion.before}><del>{suggestion.before}</del> → <ins>{suggestion.after}</ins><button type="button" onClick={() => onApplyFluency(suggestion)}>Aplicar</button></li>)}</ul></details>}
    </section>
  );
}
