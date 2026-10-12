import { forceCenter, forceCollide, forceLink, forceManyBody, forceSimulation, forceX, forceY, type SimulationLinkDatum, type SimulationNodeDatum } from "d3-force";

export interface GraphNode { id: string; weight: number; }
export interface GraphEdge { from: string; to: string; }
export interface Positioned { id: string; x: number; y: number; }

type SimNode = SimulationNodeDatum & { id: string; weight: number };

/** Raio do nó em função do grau (também usado para evitar sobreposição). */
export const nodeRadius = (degree: number) => 4 + Math.sqrt(Math.max(degree, 0)) * 1.7;

/**
 * Layout de forças com d3-force (repulsão por quadtree, mola nas ligações, colisão por raio e
 * gravidade leve para componentes soltos). Roda de forma síncrona e determinística (a posição
 * inicial do d3 é uma espiral fixa), depois ajusta o desenho ao quadro `width × height`.
 */
export function layoutGraph(nodes: GraphNode[], edges: GraphEdge[], width = 900, height = 600): Positioned[] {
  if (!nodes.length) return [];
  const simNodes: SimNode[] = nodes.map((node) => ({ id: node.id, weight: node.weight }));
  const known = new Set(simNodes.map((node) => node.id));
  const links: SimulationLinkDatum<SimNode>[] = edges.filter((edge) => known.has(edge.from) && known.has(edge.to)).map((edge) => ({ source: edge.from, target: edge.to }));
  const simulation = forceSimulation<SimNode>(simNodes)
    .force("link", forceLink<SimNode, SimulationLinkDatum<SimNode>>(links).id((node) => node.id).distance(42).strength(0.35))
    .force("charge", forceManyBody<SimNode>().strength(-70).distanceMax(320))
    .force("collide", forceCollide<SimNode>((node) => nodeRadius(node.weight) + 3))
    .force("center", forceCenter(0, 0))
    .force("x", forceX<SimNode>(0).strength(0.05))
    .force("y", forceY<SimNode>(0).strength(0.05))
    .stop();
  const ticks = Math.min(400, 120 + Math.ceil(simNodes.length / 2));
  for (let i = 0; i < ticks; i += 1) simulation.tick();

  // Ajusta o desenho ao quadro, com margem para os rótulos.
  const margin = 40;
  const xs = simNodes.map((node) => node.x ?? 0);
  const ys = simNodes.map((node) => node.y ?? 0);
  const [minX, maxX, minY, maxY] = [Math.min(...xs), Math.max(...xs), Math.min(...ys), Math.max(...ys)];
  const scale = Math.min((width - margin * 2) / Math.max(maxX - minX, 1), (height - margin * 2) / Math.max(maxY - minY, 1), 3);
  const [cx, cy] = [(minX + maxX) / 2, (minY + maxY) / 2];
  return simNodes.map((node) => ({ id: node.id, x: width / 2 + ((node.x ?? 0) - cx) * scale, y: height / 2 + ((node.y ?? 0) - cy) * scale }));
}

/** Vizinhança até `depth` saltos ao redor de `start` (ligações nos dois sentidos). */
export function neighborhood(start: string, edges: GraphEdge[], depth: number): Set<string> {
  const adjacency = new Map<string, string[]>();
  for (const edge of edges) {
    (adjacency.get(edge.from) ?? adjacency.set(edge.from, []).get(edge.from)!).push(edge.to);
    (adjacency.get(edge.to) ?? adjacency.set(edge.to, []).get(edge.to)!).push(edge.from);
  }
  const seen = new Set([start]);
  let frontier = [start];
  for (let level = 0; level < depth; level += 1) {
    const next: string[] = [];
    for (const id of frontier) for (const other of adjacency.get(id) ?? []) if (!seen.has(other)) { seen.add(other); next.push(other); }
    frontier = next;
  }
  return seen;
}
