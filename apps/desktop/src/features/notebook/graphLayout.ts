export interface GraphNode { id: string; weight: number; }
export interface GraphEdge { from: string; to: string; }
export interface Positioned { id: string; x: number; y: number; }

/** Layout de forças (Fruchterman–Reingold) determinístico, em um quadro `width × height`. */
export function layoutGraph(nodes: GraphNode[], edges: GraphEdge[], width = 900, height = 600, iterations = 220): Positioned[] {
  const n = nodes.length;
  if (!n) return [];
  const index = new Map(nodes.map((node, i) => [node.id, i]));
  // Posição inicial em círculo (determinística: o mesmo caderno gera o mesmo desenho).
  const px = nodes.map((_, i) => width / 2 + Math.cos((2 * Math.PI * i) / n) * Math.min(width, height) * 0.35);
  const py = nodes.map((_, i) => height / 2 + Math.sin((2 * Math.PI * i) / n) * Math.min(width, height) * 0.35);
  const area = width * height;
  const k = Math.sqrt(area / Math.max(n, 1)) * 0.8;
  const links = edges.map((edge) => [index.get(edge.from), index.get(edge.to)] as const).filter((pair): pair is readonly [number, number] => pair[0] !== undefined && pair[1] !== undefined);
  let temperature = Math.min(width, height) / 8;
  for (let step = 0; step < iterations; step += 1) {
    const dx = new Array<number>(n).fill(0);
    const dy = new Array<number>(n).fill(0);
    for (let a = 0; a < n; a += 1) {
      for (let b = a + 1; b < n; b += 1) {
        let vx = px[a] - px[b];
        let vy = py[a] - py[b];
        let distance = Math.hypot(vx, vy);
        if (distance < 0.01) { vx = (a % 7) - 3 + 0.1; vy = (b % 5) - 2 + 0.1; distance = Math.hypot(vx, vy); }
        const force = (k * k) / distance;
        dx[a] += (vx / distance) * force; dy[a] += (vy / distance) * force;
        dx[b] -= (vx / distance) * force; dy[b] -= (vy / distance) * force;
      }
    }
    for (const [a, b] of links) {
      const vx = px[a] - px[b];
      const vy = py[a] - py[b];
      const distance = Math.max(Math.hypot(vx, vy), 0.01);
      const force = (distance * distance) / k;
      dx[a] -= (vx / distance) * force; dy[a] -= (vy / distance) * force;
      dx[b] += (vx / distance) * force; dy[b] += (vy / distance) * force;
    }
    for (let i = 0; i < n; i += 1) {
      // Gravidade leve ao centro evita que componentes soltos fujam do quadro.
      dx[i] += (width / 2 - px[i]) * 0.02; dy[i] += (height / 2 - py[i]) * 0.02;
      const length = Math.max(Math.hypot(dx[i], dy[i]), 0.01);
      const move = Math.min(length, temperature);
      px[i] = Math.min(width - 30, Math.max(30, px[i] + (dx[i] / length) * move));
      py[i] = Math.min(height - 30, Math.max(30, py[i] + (dy[i] / length) * move));
    }
    temperature *= 0.985;
  }
  return nodes.map((node, i) => ({ id: node.id, x: px[i], y: py[i] }));
}
