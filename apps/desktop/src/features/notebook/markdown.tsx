import type { ReactNode } from "react";
import { splitFrontmatter } from "./notebookApi";

interface RenderOptions {
  /** Devolve o caminho da nota para um alvo de wikilink, ou `null` se não existir. */
  resolve: (target: string) => string | null;
  onOpen: (path: string) => void;
  onCreate: (target: string) => void;
}

// Renderizador mínimo e SEGURO: só produz elementos React (nunca HTML bruto).
function inline(text: string, options: RenderOptions, keyPrefix: string): ReactNode[] {
  const nodes: ReactNode[] = [];
  const pattern = /\[\[([^\[\]\n|#]+)(#[^\[\]\n|]*)?(?:\|([^\[\]\n]*))?\]\]|\*\*([^*\n]+)\*\*|\*([^*\n]+)\*|`([^`\n]+)`/gu;
  let last = 0;
  let index = 0;
  for (const match of text.matchAll(pattern)) {
    const start = match.index ?? 0;
    if (start > last) nodes.push(text.slice(last, start));
    const key = `${keyPrefix}-${index++}`;
    if (match[1] !== undefined) {
      const target = match[1].trim();
      const label = (match[3] ?? target).trim();
      const path = options.resolve(target);
      nodes.push(path
        ? <a key={key} className="wikilink" href="#" onClick={(event) => { event.preventDefault(); options.onOpen(path); }}>{label}</a>
        : <a key={key} className="wikilink broken" href="#" title="Nota inexistente: clique para criar" onClick={(event) => { event.preventDefault(); options.onCreate(target); }}>{label}</a>);
    } else if (match[4] !== undefined) nodes.push(<strong key={key}>{match[4]}</strong>);
    else if (match[5] !== undefined) nodes.push(<em key={key}>{match[5]}</em>);
    else if (match[6] !== undefined) nodes.push(<code key={key}>{match[6]}</code>);
    last = start + match[0].length;
  }
  if (last < text.length) nodes.push(text.slice(last));
  return nodes;
}

export function renderMarkdown(source: string, options: RenderOptions): ReactNode[] {
  const { body } = splitFrontmatter(source);
  const blocks: ReactNode[] = [];
  const lines = body.split(/\r?\n/u);
  let list: { ordered: boolean; items: string[] } | null = null;
  let paragraph: string[] = [];
  const flushParagraph = () => { if (paragraph.length) { blocks.push(<p key={`p${blocks.length}`}>{inline(paragraph.join(" "), options, `p${blocks.length}`)}</p>); paragraph = []; } };
  const flushList = () => {
    if (!list) return;
    const Tag = list.ordered ? "ol" : "ul";
    blocks.push(<Tag key={`l${blocks.length}`}>{list.items.map((item, i) => <li key={i}>{inline(item, options, `l${blocks.length}-${i}`)}</li>)}</Tag>);
    list = null;
  };
  for (const line of lines) {
    const heading = /^(#{1,4})\s+(.*)$/u.exec(line);
    const bullet = /^\s*[-*]\s+(.*)$/u.exec(line);
    const numbered = /^\s*\d+[.)]\s+(.*)$/u.exec(line);
    if (heading) { flushParagraph(); flushList(); const Tag = `h${Math.min(heading[1].length + 1, 5)}` as "h2" | "h3" | "h4" | "h5"; blocks.push(<Tag key={`h${blocks.length}`}>{inline(heading[2], options, `h${blocks.length}`)}</Tag>); }
    else if (bullet || numbered) { flushParagraph(); const ordered = Boolean(numbered); if (!list || list.ordered !== ordered) { flushList(); list = { ordered, items: [] }; } list.items.push((bullet ?? numbered)![1]); }
    else if (/^\s*>\s?/u.test(line)) { flushParagraph(); flushList(); blocks.push(<blockquote key={`q${blocks.length}`}>{inline(line.replace(/^\s*>\s?/u, ""), options, `q${blocks.length}`)}</blockquote>); }
    else if (/^\s*---+\s*$/u.test(line)) { flushParagraph(); flushList(); blocks.push(<hr key={`r${blocks.length}`} />); }
    else if (!line.trim()) { flushParagraph(); flushList(); }
    else { flushList(); paragraph.push(line.trim()); }
  }
  flushParagraph(); flushList();
  return blocks;
}
