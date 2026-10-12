import { useEffect, useRef, useState } from "react";
import type { ChapterText, FidelityReport } from "../skill/skillApi";
import "./Translation.css";

interface OriginalPaneProps {
  chapters: ChapterText[];
  index: number;
  isAuto: boolean;
  onSelect: (index: number) => void;
  sourceLabel: string;
  page: number;
  pageCount: number;
  report: FidelityReport | null;
}

export function OriginalPane({ chapters, index, isAuto, onSelect, sourceLabel, page, pageCount, report }: OriginalPaneProps) {
  const bodyRef = useRef<HTMLDivElement>(null);
  const [sync, setSync] = useState(true);
  const lines = chapters[index]?.lines ?? [];
  const flagged = new Map((report?.aligned ? report.flags : []).map((flag) => [flag.index, flag.kind]));

  // A rolagem do original acompanha a página aberta na tradução (proporcionalmente).
  useEffect(() => {
    const body = bodyRef.current;
    if (!sync || !body) return;
    const fraction = pageCount > 1 ? (page - 1) / (pageCount - 1) : 0;
    body.scrollTo({ top: fraction * Math.max(0, body.scrollHeight - body.clientHeight), behavior: "smooth" });
  }, [page, pageCount, sync, index]);

  return (
    <section className="original-pane" aria-label="Texto original">
      <header>
        <div><span className="eyebrow">ORIGINAL</span><strong title={sourceLabel}>{sourceLabel}</strong></div>
        <label className="sync-toggle"><input type="checkbox" checked={sync} onChange={(event) => setSync(event.target.checked)} /> acompanhar página</label>
      </header>
      <select value={index} onChange={(event) => onSelect(Number(event.target.value))} aria-label="Capítulo do original">
        {chapters.map((chapter, position) => <option key={`${chapter.name}-${position}`} value={position}>{String(position + 1).padStart(2, "0")} · {chapter.name}</option>)}
      </select>
      <small className="pane-hint">{isAuto ? "Capítulo escolhido pelo nome do arquivo (ou pela posição)." : "Capítulo escolhido manualmente."} {lines.length} parágrafos.</small>
      <div className="original-body" ref={bodyRef}>
        {lines.map((line, position) => <p key={position} className={flagged.has(position) ? `flag-${flagged.get(position)}` : ""}><span>{position + 1}</span>{line}</p>)}
      </div>
    </section>
  );
}
