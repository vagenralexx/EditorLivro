import { useCallback, useEffect, useMemo, useRef, useState, type CSSProperties, type MouseEvent as ReactMouseEvent, type PointerEvent as ReactPointerEvent, type WheelEvent as ReactWheelEvent } from "react";
import { layoutGraph, neighborhood, nodeRadius } from "./graphLayout";
import { kindInfo, kindOrder, type NoteKind, type NoteMeta } from "./notebookApi";

interface GraphViewProps {
  notes: NoteMeta[];
  edges: { from: string; to: string }[];
  selected: string;
  onOpen: (path: string) => void;
}

const FRAME = { width: 900, height: 600 };
const LABELLED_BY_DEFAULT = 16;

export function GraphView({ notes, edges, selected, onOpen }: GraphViewProps) {
  const [hiddenKinds, setHiddenKinds] = useState<Set<NoteKind>>(new Set());
  const [mode, setMode] = useState<"tudo" | "vizinhanca">(selected ? "vizinhanca" : "tudo");
  const litRef = useRef<SVGPathElement>(null);
  const hoveredRef = useRef<{ id: string; touched: Element[] } | null>(null);
  const [view, setView] = useState({ x: 0, y: 0, w: FRAME.width, h: FRAME.height });
  const drag = useRef<{ x: number; y: number; view: typeof view } | null>(null);
  const svgRef = useRef<SVGSVGElement>(null);
  const selectedMeta = notes.find((note) => note.path === selected);
  const effectiveMode = mode === "vizinhanca" && selectedMeta ? "vizinhanca" : "tudo";

  // Nós e ligações visíveis: o layout só é recalculado quando ESTES mudam (não a cada movimento do mouse).
  const visible = useMemo(() => {
    const byKind = notes.filter((note) => !hiddenKinds.has(note.kind));
    const allowed = effectiveMode === "vizinhanca" ? neighborhood(selected, edges, 2) : null;
    const nodes = allowed ? byKind.filter((note) => allowed.has(note.path)) : byKind;
    const ids = new Set(nodes.map((note) => note.path));
    const links = edges.filter((edge) => ids.has(edge.from) && ids.has(edge.to));
    const degree = new Map<string, number>();
    for (const edge of links) { degree.set(edge.from, (degree.get(edge.from) ?? 0) + 1); degree.set(edge.to, (degree.get(edge.to) ?? 0) + 1); }
    return { nodes, links, degree };
  }, [edges, effectiveMode, hiddenKinds, notes, selected]);

  const positions = useMemo(() => new Map(layoutGraph(visible.nodes.map((note) => ({ id: note.path, weight: visible.degree.get(note.path) ?? 0 })), visible.links, FRAME.width, FRAME.height).map((p) => [p.id, p])), [visible]);

  useEffect(() => { setView({ x: 0, y: 0, w: FRAME.width, h: FRAME.height }); }, [positions]);

  // Vizinhança e ligações por nó: o destaque do mouse é feito direto no DOM (sem re-render do React).
  const adjacency = useMemo(() => {
    const map = new Map<string, { id: string; segment: string }[]>();
    for (const edge of visible.links) {
      const a = positions.get(edge.from);
      const b = positions.get(edge.to);
      if (!a || !b) continue;
      const segment = `M${a.x.toFixed(1)} ${a.y.toFixed(1)}L${b.x.toFixed(1)} ${b.y.toFixed(1)}`;
      (map.get(edge.from) ?? map.set(edge.from, []).get(edge.from)!).push({ id: edge.to, segment });
      (map.get(edge.to) ?? map.set(edge.to, []).get(edge.to)!).push({ id: edge.from, segment });
    }
    return map;
  }, [positions, visible.links]);

  // Todas as ligações em UM <path>: o DOM não cresce com o número de ligações.
  const basePath = useMemo(() => visible.links.map((edge) => { const a = positions.get(edge.from); const b = positions.get(edge.to); return a && b ? `M${a.x.toFixed(1)} ${a.y.toFixed(1)}L${b.x.toFixed(1)} ${b.y.toFixed(1)}` : ""; }).join(""), [positions, visible.links]);

  const clearHover = useCallback(() => {
    const current = hoveredRef.current;
    if (!current) return;
    for (const element of current.touched) element.classList.remove("hov", "nb");
    litRef.current?.setAttribute("d", "");
    svgRef.current?.classList.remove("hovering");
    hoveredRef.current = null;
  }, []);

  const applyHover = useCallback((id: string) => {
    if (hoveredRef.current?.id === id) return;
    clearHover();
    const svg = svgRef.current;
    if (!svg) return;
    const links = adjacency.get(id) ?? [];
    const touched: Element[] = [];
    const mark = (nodeId: string, className: string) => { const element = svg.querySelector(`[data-id="${CSS.escape(nodeId)}"]`); if (element) { element.classList.add(className); touched.push(element); } };
    mark(id, "hov");
    for (const link of links) mark(link.id, "nb");
    litRef.current?.setAttribute("d", links.map((link) => link.segment).join(""));
    svg.classList.add("hovering");
    hoveredRef.current = { id, touched };
  }, [adjacency, clearHover]);

  // Trocar de modo/filtro recria os nós: o destaque antigo deixa de existir.
  useEffect(() => { clearHover(); }, [clearHover, positions]);

  const onOver = (event: ReactMouseEvent<SVGSVGElement>) => { const node = (event.target as Element).closest<SVGGElement>(".node"); if (node?.dataset.id) applyHover(node.dataset.id); };
  const onOut = (event: ReactMouseEvent<SVGSVGElement>) => { if (!(event.relatedTarget as Element | null)?.closest?.(".node")) clearHover(); };

  const topLabelled = useMemo(() => new Set([...visible.degree.entries()].sort((a, b) => b[1] - a[1]).slice(0, LABELLED_BY_DEFAULT).map(([id]) => id)), [visible.degree]);
  const zoom = FRAME.width / view.w;
  const svgStyle = { "--label-size": `${(11 / Math.max(zoom, 0.6)).toFixed(2)}px` } as CSSProperties;

  const toggleKind = (kind: NoteKind) => setHiddenKinds((current) => { const next = new Set(current); if (next.has(kind)) next.delete(kind); else next.add(kind); return next; });

  const onWheel = useCallback((event: ReactWheelEvent<SVGSVGElement>) => {
    const svg = svgRef.current;
    if (!svg) return;
    const rect = svg.getBoundingClientRect();
    const factor = event.deltaY > 0 ? 1.15 : 1 / 1.15;
    setView((current) => {
      const w = Math.min(FRAME.width * 1.5, Math.max(FRAME.width / 8, current.w * factor));
      const h = w * (FRAME.height / FRAME.width);
      const px = current.x + ((event.clientX - rect.left) / rect.width) * current.w;
      const py = current.y + ((event.clientY - rect.top) / rect.height) * current.h;
      return { w, h, x: px - ((event.clientX - rect.left) / rect.width) * w, y: py - ((event.clientY - rect.top) / rect.height) * h };
    });
  }, []);

  const onPointerDown = (event: ReactPointerEvent<SVGSVGElement>) => {
    if ((event.target as Element).closest(".node")) return;
    drag.current = { x: event.clientX, y: event.clientY, view };
    (event.currentTarget as SVGSVGElement).setPointerCapture(event.pointerId);
  };
  const onPointerMove = (event: ReactPointerEvent<SVGSVGElement>) => {
    const start = drag.current;
    const svg = svgRef.current;
    if (!start || !svg) return;
    const rect = svg.getBoundingClientRect();
    setView({ ...start.view, x: start.view.x - ((event.clientX - start.x) / rect.width) * start.view.w, y: start.view.y - ((event.clientY - start.y) / rect.height) * start.view.h });
  };
  const endDrag = () => { drag.current = null; };

  const nodeLayer = useMemo(() => visible.nodes.map((note) => {
    const p = positions.get(note.path);
    if (!p) return null;
    const r = nodeRadius(visible.degree.get(note.path) ?? 0);
    const label = note.heading ?? note.title;
    return (
      <g key={note.path} data-id={note.path} className={`node${note.path === selected ? " sel" : ""}${topLabelled.has(note.path) ? " top" : ""}`} transform={`translate(${p.x.toFixed(1)},${p.y.toFixed(1)})`} onClick={() => onOpen(note.path)} tabIndex={0} role="button" aria-label={label} onKeyDown={(event) => { if (event.key === "Enter") onOpen(note.path); }}>
        <circle r={r} fill={kindInfo[note.kind].color} />
        <text y={r} dy="1.25em" textAnchor="middle">{label.length > 26 ? `${label.slice(0, 25)}…` : label}</text>
      </g>
    );
  }), [onOpen, positions, selected, topLabelled, visible.degree, visible.nodes]);

  if (notes.length === 0) return <p className="notebook-hint">O caderno ainda não tem notas para mostrar no grafo.</p>;

  return (
    <div className="graph-view">
      <div className="graph-filters">
        <div className="graph-mode" role="group" aria-label="O que mostrar">
          <button type="button" aria-pressed={effectiveMode === "tudo"} className={effectiveMode === "tudo" ? "active" : ""} onClick={() => setMode("tudo")}>Todas as notas</button>
          <button type="button" aria-pressed={effectiveMode === "vizinhanca"} className={effectiveMode === "vizinhanca" ? "active" : ""} disabled={!selectedMeta} onClick={() => setMode("vizinhanca")} title={selectedMeta ? "A nota aberta e o que está a até 2 ligações dela" : "Abra uma nota na aba Notas para ver a vizinhança dela"}>Vizinhança de {selectedMeta ? `“${(selectedMeta.heading ?? selectedMeta.title).slice(0, 24)}”` : "uma nota"}</button>
        </div>
        {kindOrder.map((kind) => { const count = notes.filter((note) => note.kind === kind).length; if (!count) return null; const off = hiddenKinds.has(kind); return <button key={kind} type="button" aria-pressed={!off} className={off ? "off" : ""} onClick={() => toggleKind(kind)}><span className="kind-dot" style={{ background: kindInfo[kind].color }} />{kindInfo[kind].plural} ({count})</button>; })}
        <button type="button" className="graph-reset" onClick={() => setView({ x: 0, y: 0, w: FRAME.width, h: FRAME.height })} disabled={view.w === FRAME.width && view.x === 0 && view.y === 0}>Reiniciar zoom</button>
      </div>
      <svg ref={svgRef} viewBox={`${view.x} ${view.y} ${view.w} ${view.h}`} style={svgStyle} role="group" aria-label="Grafo de notas" onWheel={onWheel} onPointerDown={onPointerDown} onPointerMove={onPointerMove} onPointerUp={endDrag} onPointerCancel={endDrag} onMouseOver={onOver} onMouseOut={onOut} onFocus={(event) => { const node = (event.target as Element).closest<SVGGElement>(".node"); if (node?.dataset.id) applyHover(node.dataset.id); }} onBlur={clearHover} className={`${zoom >= 2.2 ? "zoomed" : ""}`}>
        <path d={basePath} className="edges base" />
        <path ref={litRef} className="edges lit" d="" />
        {nodeLayer}
      </svg>
      <p className="muted">{visible.nodes.length} notas · {visible.links.length} ligações. Role para aproximar, arraste o fundo para mover, passe o mouse para destacar os vizinhos e clique para abrir. Notas sem links ficam isoladas — ligue-as com <code>[[ ]]</code>.</p>
    </div>
  );
}
