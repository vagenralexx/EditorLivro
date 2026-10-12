/**
 * Aplica uma sugestão "antes → depois" da IA no XHTML.
 *
 * A IA recebe o texto corrido do capítulo (sem tags, entidades decodificadas, um parágrafo por linha) e
 * devolve o trecho "antes" nesse formato. Procurar esse trecho direto no XHTML falha sempre que ele
 * cruza uma tag (<em>, mudança de parágrafo) ou tem uma entidade (&amp;, &#8217;). Aqui a busca é feita no
 * texto visível, e a troca, nos nós de texto: o que é marcação fora do trecho continua intacto.
 */

const NAMED: Record<string, string> = {
  amp: "&", lt: "<", gt: ">", quot: '"', apos: "'", nbsp: " ", hellip: "…", mdash: "—", ndash: "–",
  rsquo: "’", lsquo: "‘", rdquo: "”", ldquo: "“", laquo: "«", raquo: "»",
};
const BLOCK = new Set(["p", "h1", "h2", "h3", "h4", "h5", "h6", "li", "blockquote", "div", "section", "tr", "br", "hr", "ul", "ol", "table"]);

interface Piece { ch: string; segment: number; start: number; end: number; }

function decodeEntity(raw: string): string | null {
  const match = /^&(?:#(\d+)|#[xX]([0-9a-fA-F]+)|([A-Za-z][A-Za-z0-9]*));$/u.exec(raw);
  if (!match) return null;
  if (match[1] || match[2]) {
    const code = match[1] ? Number(match[1]) : parseInt(match[2], 16);
    return Number.isFinite(code) && code > 0 && code <= 0x10ffff ? String.fromCodePoint(code) : null;
  }
  return NAMED[match[3]] ?? null;
}

const fold = (ch: string) => (ch === "’" || ch === "‘" ? "'" : ch === "“" || ch === "”" ? '"' : /\s/u.test(ch) ? " " : ch);

function normalize(text: string): string {
  let out = "";
  for (const ch of text) {
    const c = fold(ch);
    if (c === " " && (out === "" || out.endsWith(" "))) continue;
    out += c;
  }
  return out.endsWith(" ") ? out.slice(0, -1) : out;
}

const escapeText = (text: string) => text.replace(/&/gu, "&amp;").replace(/</gu, "&lt;").replace(/>/gu, "&gt;");

/** Texto visível, caractere a caractere, com o intervalo de cada um no XHTML. */
function visibleText(xhtml: string): Piece[] {
  const pieces: Piece[] = [];
  let segmentCount = 0;
  const bodyStart = /<body\b[^>]*>/iu.exec(xhtml);
  let i = bodyStart ? bodyStart.index + bodyStart[0].length : 0;
  const pushSpace = (segment: number) => { if (pieces.length && pieces[pieces.length - 1].ch !== " ") pieces.push({ ch: " ", segment, start: -1, end: -1 }); };
  while (i < xhtml.length) {
    if (xhtml[i] === "<") {
      let end: number;
      if (xhtml.startsWith("<!--", i)) end = xhtml.indexOf("-->", i) + 3;
      else if (xhtml.startsWith("<![CDATA[", i)) end = xhtml.indexOf("]]>", i) + 3;
      else end = xhtml.indexOf(">", i) + 1;
      if (end <= i) break;
      const name = /^<\/?([A-Za-z][A-Za-z0-9]*)/u.exec(xhtml.slice(i, end))?.[1]?.toLowerCase();
      if (name && BLOCK.has(name)) pushSpace(-1);
      i = end;
      continue;
    }
    const segment = segmentCount++;
    while (i < xhtml.length && xhtml[i] !== "<") {
      let raw = xhtml[i];
      let length = 1;
      if (raw === "&") {
        const semi = xhtml.indexOf(";", i);
        if (semi > i && semi - i <= 10) { const decoded = decodeEntity(xhtml.slice(i, semi + 1)); if (decoded !== null) { raw = decoded; length = semi - i + 1; } }
      }
      const ch = fold(raw);
      if (ch === " ") pushSpace(segment);
      else pieces.push({ ch, segment, start: i, end: i + length });
      i += length;
    }
  }
  return pieces;
}

/** Remove os elementos que ficaram vazios (`<em></em>`, `<p> </p>`) colados ao ponto de edição. */
function dropEmptyAround(xhtml: string, at: number): string {
  let text = xhtml;
  for (;;) {
    const before = /<([A-Za-z][A-Za-z0-9]*)(?:\s[^>]*)?(?<!\/)>\s*$/u.exec(text.slice(0, at));
    if (!before) break;
    const after = new RegExp(`^\\s*</${before[1]}\\s*>`, "u").exec(text.slice(at));
    if (!after) break;
    text = text.slice(0, before.index) + text.slice(at + after[0].length);
    at = before.index;
  }
  return text;
}

/** Devolve o XHTML com a sugestão aplicada, ou `null` se o trecho "antes" não existe no texto. */
export function applyTextSuggestion(xhtml: string, beforeText: string, afterText: string): string | null {
  const needle = normalize(beforeText);
  if (!needle) return null;
  const pieces = visibleText(xhtml);
  const haystack = pieces.map((piece) => piece.ch).join("");
  const index = haystack.indexOf(needle);
  if (index < 0) return null;

  const matched = pieces.slice(index, index + needle.length).filter((piece) => piece.start >= 0);
  if (!matched.length) return null;
  // Intervalo bruto a trocar em cada nó de texto tocado, do primeiro ao último.
  const ranges = new Map<number, { start: number; end: number }>();
  for (const piece of matched) {
    const range = ranges.get(piece.segment);
    if (!range) ranges.set(piece.segment, { start: piece.start, end: piece.end });
    else { range.start = Math.min(range.start, piece.start); range.end = Math.max(range.end, piece.end); }
  }
  const order = [...ranges.keys()].sort((a, b) => a - b);
  const first = order[0];
  let result = xhtml;
  // De trás para frente: os deslocamentos não afetam os intervalos que ainda faltam.
  for (const segment of [...order].reverse()) {
    const { start, end } = ranges.get(segment)!;
    const insert = segment === first ? escapeText(afterText) : "";
    result = result.slice(0, start) + insert + result.slice(end);
    if (!insert) result = dropEmptyAround(result, start);
  }
  return result;
}
