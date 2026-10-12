import type { Genesis } from "./skillApi";
import "./SkillPanels.css";

export function GenesisCard({ genesis, genreLabel }: { genesis: Genesis; genreLabel: string }) {
  return (
    <div className="genesis-card">
      <header><strong>Genesis Score {genesis.score.toFixed(1)}</strong><span className={genesis.approved ? "ok" : "warn"}>{genesis.approved ? "APROVADO" : "REVISÃO NECESSÁRIA"} (limiar {genesis.threshold.toFixed(1)} · {genreLabel})</span></header>
      <table><tbody>{genesis.dimensions.map((dimension) => <tr key={dimension.dimension} className={dimension.score === genesis.floor ? "floor" : ""}><th>{dimension.dimension}</th><td>{dimension.score.toFixed(1)}</td><td>{dimension.evidence}</td></tr>)}</tbody></table>
      <small>Floor {genesis.floor.toFixed(1)} × 0,4 + Média {genesis.mean.toFixed(1)} × 0,6. As notas vêm da IA; a conta é feita pelo aplicativo.</small>
    </div>
  );
}
