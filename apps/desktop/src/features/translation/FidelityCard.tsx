import type { FidelityReport } from "../skill/skillApi";
import "./Translation.css";

const statusClass = (status: string) => (status === "ok" ? "ok" : "bad");

export function FidelityCard({ report }: { report: FidelityReport }) {
  const problems = report.terms.filter((term) => term.status !== "ok").length + report.rawLanguage.filter((raw) => raw.status !== "ok").length + report.flags.length + report.notes.length;
  return (
    <section className="fidelity-card" aria-label="Fidelidade original × tradução">
      <div className="scan-title"><strong>Fidelidade original × tradução</strong><span>sem IA · capítulo inteiro</span></div>
      <div className="scan-grid">
        <div><b>{report.originalParagraphs}→{report.translatedParagraphs}</b><small>parágrafos</small></div>
        <div className={report.originalDialogues === report.translatedDialogues ? "" : "warn"}><b>{report.originalDialogues}→{report.translatedDialogues}</b><small>falas</small></div>
        <div className={report.expansion >= 0.85 && report.expansion <= 1.5 ? "" : "warn"}><b>{(report.expansion * 100).toFixed(0)}%</b><small>palavras vs. original</small></div>
        <div className={problems ? "warn" : ""}><b>{problems}</b><small>pontos a conferir</small></div>
      </div>
      {report.notes.length > 0 && <ul className="scan-notes">{report.notes.map((note, index) => <li key={index}>{note}</li>)}</ul>}
      {report.rawLanguage.length > 0 && <details className="scan-details" open={report.rawLanguage.some((raw) => raw.status !== "ok")}><summary>Linguagem crua preservada?</summary>
        <ul>{report.rawLanguage.map((raw) => <li key={raw.raw}><b>{raw.raw}</b> {raw.original}× no original → {raw.translated}× em {raw.equivalents.join("/")} <em className={statusClass(raw.status)}>{raw.status}</em></li>)}</ul></details>}
      {report.terms.length > 0 && <details className="scan-details" open={report.terms.some((term) => term.status !== "ok")}><summary>Glossário respeitado?</summary>
        <ul>{report.terms.map((term) => <li key={term.term}><b>{term.term}</b> {term.original}× → {term.translated}× <em className={statusClass(term.status)}>{term.status}</em></li>)}</ul></details>}
      {report.flags.length > 0 && <details className="scan-details" open><summary>Parágrafos suspeitos ({report.flags.length})</summary>
        <ul>{report.flags.map((flag) => <li key={flag.index}><b>#{flag.index + 1} · possível {flag.kind === "omissao" ? "omissão" : "adição"}</b> ({(flag.ratio * 100).toFixed(0)}% do tamanho)<q>{flag.original}</q><q>{flag.translated}</q></li>)}</ul></details>}
      {!report.aligned && <p className="scan-muted">Sem o mesmo número de parágrafos, a checagem parágrafo a parágrafo fica desativada; as contagens por capítulo continuam valendo.</p>}
    </section>
  );
}
