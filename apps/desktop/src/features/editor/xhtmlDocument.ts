export interface XhtmlShell {
  declaration: string;
  doctype: string;
  htmlOpenTag: string;
  headHtml: string;
  bodyOpenTag: string;
}

export interface EditableXhtmlDocument {
  shell: XhtmlShell;
  bodyHtml: string;
}

const fallbackShell: XhtmlShell = {
  declaration: '<?xml version="1.0" encoding="UTF-8"?>',
  doctype: "<!DOCTYPE html>",
  htmlOpenTag: '<html xmlns="http://www.w3.org/1999/xhtml" lang="pt-BR">',
  headHtml: '<meta charset="UTF-8" /><title>Capítulo</title>',
  bodyOpenTag: "<body>",
};

function findOpenTag(source: string, expression: RegExp, fallback: string) {
  return source.match(expression)?.[0] ?? fallback;
}

function removeUnsafeMarkup(html: string) {
  return html
    .replace(/<(?:script|iframe|object|embed|form)\b[^>]*>[\s\S]*?<\/(?:script|iframe|object|embed|form)>/giu, "")
    .replace(/\son\w+\s*=\s*(?:"[^"]*"|'[^']*'|[^\s>]+)/giu, "")
    .replace(/\s(?:href|src)\s*=\s*(?:"\s*javascript:[^"]*"|'\s*javascript:[^']*'|javascript:[^\s>]+)/giu, "");
}

/**
 * Splits an XHTML document without normalising the existing markup. This is
 * intentional: a book can carry CSS classes and EPUB semantics a generic rich
 * text model does not know about.
 */
export function parseEditableXhtml(source: string): EditableXhtmlDocument {
  const bodyStart = /<body\b[^>]*>/iu.exec(source);
  const bodyEnd = /<\/body\s*>/giu;
  const closingMatches = [...source.matchAll(bodyEnd)];
  const lastBodyEnd = closingMatches[closingMatches.length - 1];

  if (!bodyStart || !lastBodyEnd || lastBodyEnd.index === undefined) {
    return { shell: fallbackShell, bodyHtml: removeUnsafeMarkup(source) };
  }

  const headStart = /<head\b[^>]*>/iu.exec(source);
  const headEnd = /<\/head\s*>/iu.exec(source);
  const declaration = source.match(/^\s*(<\?xml[^?]*\?>)/iu)?.[1] ?? fallbackShell.declaration;
  const doctype = source.match(/<!doctype[^>]*>/iu)?.[0] ?? fallbackShell.doctype;

  return {
    shell: {
      declaration,
      doctype,
      htmlOpenTag: findOpenTag(source, /<html\b[^>]*>/iu, fallbackShell.htmlOpenTag),
      headHtml: headStart && headEnd && headEnd.index !== undefined
        ? source.slice(headStart.index + headStart[0].length, headEnd.index)
        : fallbackShell.headHtml,
      bodyOpenTag: bodyStart[0],
    },
    bodyHtml: removeUnsafeMarkup(source.slice(bodyStart.index + bodyStart[0].length, lastBodyEnd.index)),
  };
}

function toXhtmlVoidElements(html: string) {
  return html.replace(/<(br|hr|img|meta|link|input)(\b[^>]*)>/giu, (_whole, tag: string, attributes: string) => {
    const normalisedAttributes = attributes.trimEnd().replace(/\/$/u, "").trimEnd();
    return `<${tag}${normalisedAttributes ? ` ${normalisedAttributes}` : ""} />`;
  });
}

/**
 * `innerHTML` serializa espaços não separáveis como `&nbsp;`, entidade que não
 * existe em XML: o XHTML resultante seria rejeitado por leitores e validadores.
 */
function toXmlSafeEntities(html: string) {
  return html.replace(/&nbsp;/gu, "&#160;");
}

export function composeEditableXhtml(shell: XhtmlShell, bodyHtml: string) {
  return [
    shell.declaration,
    shell.doctype,
    shell.htmlOpenTag,
    "<head>",
    shell.headHtml.trim(),
    "</head>",
    shell.bodyOpenTag,
    toXhtmlVoidElements(toXmlSafeEntities(bodyHtml.trim())),
    "</body>",
    "</html>",
    "",
  ].join("\n");
}

export function textFromXhtml(source: string) {
  const document = parseEditableXhtml(source);
  const container = globalThis.document?.createElement("div");
  if (!container) return document.bodyHtml.replace(/<[^>]*>/gu, " ").replace(/\s+/gu, " ").trim();
  container.innerHTML = document.bodyHtml;
  return (container.textContent ?? "").replace(/\s+/gu, " ").trim();
}
