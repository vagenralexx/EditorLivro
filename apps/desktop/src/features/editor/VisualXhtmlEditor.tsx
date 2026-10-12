import { useCallback, useEffect, useMemo, useRef, useState, type CSSProperties } from "react";
import { composeEditableXhtml, parseEditableXhtml } from "./xhtmlDocument";
import "./VisualXhtmlEditor.css";

export type ReaderTheme = "warm" | "paper" | "night";

export interface PaginationState {
  page: number;
  pageCount: number;
  xhtml: string;
}

interface VisualXhtmlEditorProps {
  documentId?: string;
  documentName?: string;
  source: string;
  theme: ReaderTheme;
  fontScale: number;
  brightness: number;
  externalRevision: number;
  initialPage: number;
  onChange: (source: string) => void;
  onPaginationChange: (state: PaginationState) => void;
  hasPreviousChapter: boolean;
  hasNextChapter: boolean;
  onPreviousChapter: () => void;
  onNextChapter: () => void;
  onSaveRequested: () => void;
}

type FormatAction = "bold" | "italic" | "underline" | "strikeThrough" | "insertUnorderedList" | "insertOrderedList" | "undo" | "redo" | "justifyLeft" | "justifyCenter" | "justifyRight" | "justifyFull" | "indent" | "outdent";
type EditorMode = "edit" | "preview";

function displayName(documentName?: string) {
  return documentName?.replace(/\.x?html$/iu, "") || "Capítulo aberto";
}

const PAGE_GAP = 56;

export function VisualXhtmlEditor({ documentId, documentName, source, theme, fontScale, brightness, externalRevision, initialPage, onChange, onPaginationChange, hasPreviousChapter, hasNextChapter, onPreviousChapter, onNextChapter, onSaveRequested }: VisualXhtmlEditorProps) {
  const hasDocument = Boolean(documentId);
  const canvasRef = useRef<HTMLDivElement>(null);
  const initialPageRef = useRef(initialPage);
  const pendingRestorePageRef = useRef<number | undefined>(undefined);
  const pointerHeldRef = useRef(false);
  const pageRef = useRef(1);
  const navigationRef = useRef({ forward: () => {}, backward: () => {} });
  const [page, setPage] = useState(1);
  const [pageCount, setPageCount] = useState(1);
  const [columnWidth, setColumnWidth] = useState(520);
  const [mode, setMode] = useState<EditorMode>("edit");
  const shell = useMemo(() => parseEditableXhtml(source).shell, [documentId]);
  const initialBody = useMemo(() => parseEditableXhtml(source).bodyHtml, [documentId, externalRevision]);

  useEffect(() => { initialPageRef.current = initialPage; }, [initialPage]);

  useEffect(() => {
    if (canvasRef.current) {
      canvasRef.current.innerHTML = initialBody;
      canvasRef.current.scrollLeft = 0;
      setPage(1);
      pendingRestorePageRef.current = initialPageRef.current;
    }
  }, [initialBody]);

  useEffect(() => {
    const handleShortcut = (event: KeyboardEvent) => {
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "s") {
        event.preventDefault();
        onSaveRequested();
      }
    };
    window.addEventListener("keydown", handleShortcut);
    return () => window.removeEventListener("keydown", handleShortcut);
  }, [onSaveRequested]);

  const pageStep = useCallback(() => columnWidth + PAGE_GAP, [columnWidth]);

  const composePageXhtml = useCallback((currentPage: number) => {
    const canvas = canvasRef.current;
    if (!canvas) return composeEditableXhtml(shell, "");
    const start = (currentPage - 1) * pageStep();
    const end = start + columnWidth;
    const canvasRect = canvas.getBoundingClientRect();
    const visibleBlocks = Array.from(canvas.children).filter((element) => {
      const rectangles = Array.from(element.getClientRects());
      return rectangles.some((rectangle) => {
        const left = rectangle.left - canvasRect.left + canvas.scrollLeft;
        return left < end && left + rectangle.width > start;
      });
    });
    const pageHtml = visibleBlocks.length ? visibleBlocks.map((element) => element.outerHTML).join("\n") : canvas.innerHTML;
    return composeEditableXhtml(shell, pageHtml);
  }, [columnWidth, pageStep, shell]);

  const measurePagination = useCallback((requestedPage?: number) => {
    const canvas = canvasRef.current;
    if (!canvas || !documentId) return;
    const restorePage = requestedPage ?? pendingRestorePageRef.current;
    if (Math.abs(canvas.clientWidth - columnWidth) > 1) {
      // A largura da coluna mudou (ex.: Editar ⇄ Prévia Kindle, janela redimensionada): guarda a página em que
      // estávamos, calculada com o passo ANTIGO, para realinhar o scroll ao passo novo. Sem isso o scroll
      // fica no meio de uma página e o texto aparece cortado nas duas bordas.
      pendingRestorePageRef.current = restorePage ?? Math.max(1, Math.round(canvas.scrollLeft / pageStep()) + 1);
      setColumnWidth(canvas.clientWidth);
      return;
    }
    const step = pageStep();
    if (!step) return;
    const count = Math.max(1, Math.round((canvas.scrollWidth + PAGE_GAP) / step));
    const nextPage = Math.min(count, Math.max(1, restorePage ?? Math.round(canvas.scrollLeft / step) + 1));
    if (restorePage) {
      canvas.scrollLeft = (nextPage - 1) * step;
      pendingRestorePageRef.current = undefined;
    }
    setPageCount(count);
    setPage(nextPage);
    onPaginationChange({ page: nextPage, pageCount: count, xhtml: composePageXhtml(nextPage) });
  }, [columnWidth, composePageXhtml, documentId, onPaginationChange, pageStep]);

  useEffect(() => {
    const frame = window.requestAnimationFrame(() => {
      const pageToRestore = pendingRestorePageRef.current;
      pendingRestorePageRef.current = undefined;
      measurePagination(pageToRestore);
    });
    return () => window.cancelAnimationFrame(frame);
  }, [columnWidth, documentId, fontScale, initialBody, measurePagination]);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return undefined;
    const observer = new ResizeObserver(() => measurePagination());
    observer.observe(canvas);
    return () => observer.disconnect();
  }, [measurePagination]);

  const moveToPage = (nextPage: number) => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const boundedPage = Math.min(pageCount, Math.max(1, nextPage));
    canvas.scrollTo({ left: (boundedPage - 1) * pageStep(), behavior: "smooth" });
    window.setTimeout(() => measurePagination(boundedPage), 180);
  };

  const goBackward = () => {
    if (page > 1) moveToPage(page - 1);
    else onPreviousChapter();
  };

  const goForward = () => {
    if (page < pageCount) moveToPage(page + 1);
    else onNextChapter();
  };

  pageRef.current = page;
  navigationRef.current = { forward: goForward, backward: goBackward };

  // A navegação entre páginas é exclusiva das setas do app e do teclado. Gestos
  // do mouse/trackpad (arrastar seleção até a borda, rolagem horizontal, botões
  // laterais "voltar/avançar") não podem trocar de página durante a edição.
  useEffect(() => {
    const canvas = canvasRef.current;
    const release = () => { pointerHeldRef.current = false; };
    const blockWheel = (event: WheelEvent) => {
      if (event.deltaX !== 0 || event.shiftKey) event.preventDefault();
    };
    const blockSideButtons = (event: MouseEvent) => {
      if (event.button === 3 || event.button === 4) { event.preventDefault(); event.stopPropagation(); }
    };
    const handleKeys = (event: KeyboardEvent) => {
      const target = event.target as HTMLElement | null;
      if (target?.closest("input, textarea, select")) return;
      // Setas simples só navegam quando não editam texto: prévia (somente leitura) ou foco fora de campos.
      const plainArrowsNavigate = target === document.body || target?.closest(".xhtml-canvas")?.getAttribute("contenteditable") === "false";
      const forward = event.key === "PageDown" || (event.altKey && event.key === "ArrowRight") || (plainArrowsNavigate && !event.shiftKey && !event.ctrlKey && !event.metaKey && event.key === "ArrowRight");
      const backward = event.key === "PageUp" || (event.altKey && event.key === "ArrowLeft") || (plainArrowsNavigate && !event.shiftKey && !event.ctrlKey && !event.metaKey && event.key === "ArrowLeft");
      if (!forward && !backward) return;
      event.preventDefault();
      if (forward) navigationRef.current.forward(); else navigationRef.current.backward();
    };
    canvas?.addEventListener("wheel", blockWheel, { passive: false });
    window.addEventListener("mouseup", release);
    window.addEventListener("blur", release);
    for (const type of ["mousedown", "mouseup", "auxclick"] as const) window.addEventListener(type, blockSideButtons, true);
    window.addEventListener("keydown", handleKeys);
    return () => {
      canvas?.removeEventListener("wheel", blockWheel);
      window.removeEventListener("mouseup", release);
      window.removeEventListener("blur", release);
      for (const type of ["mousedown", "mouseup", "auxclick"] as const) window.removeEventListener(type, blockSideButtons, true);
      window.removeEventListener("keydown", handleKeys);
    };
  }, [hasDocument]);

  // Enquanto o botão do mouse está pressionado dentro do texto, o navegador pode
  // rolar a coluna sozinho (autoscroll da seleção). Voltamos à página atual.
  const handleCanvasScroll = () => {
    const canvas = canvasRef.current;
    if (canvas && pointerHeldRef.current) {
      const locked = (pageRef.current - 1) * pageStep();
      if (Math.abs(canvas.scrollLeft - locked) > 1) canvas.scrollTo({ left: locked, behavior: "instant" });
      return;
    }
    measurePagination();
  };

  const emitChange = () => {
    if (canvasRef.current) onChange(composeEditableXhtml(shell, canvasRef.current.innerHTML));
    window.requestAnimationFrame(() => measurePagination());
  };

  const execute = (action: FormatAction, value?: string) => {
    canvasRef.current?.focus();
    document.execCommand(action, false, value);
    emitChange();
  };

  const changeBlock = (value: string) => {
    canvasRef.current?.focus();
    document.execCommand("formatBlock", false, value);
    emitChange();
  };

  const addLink = () => {
    const url = window.prompt("Endereço do link (https://…)");
    if (!url) return;
    canvasRef.current?.focus();
    document.execCommand("createLink", false, url);
    emitChange();
  };

  const insertQuick = (kind: "dialogue" | "scene" | "thought") => {
    canvasRef.current?.focus();
    if (kind === "dialogue") document.execCommand("insertText", false, "— ");
    if (kind === "scene") document.execCommand("insertHTML", false, '<p class="scene-break">* * *</p>');
    if (kind === "thought") {
      const selection = window.getSelection();
      const range = selection?.rangeCount ? selection.getRangeAt(0) : undefined;
      if (!range || range.collapsed || !canvasRef.current?.contains(range.commonAncestorContainer)) return;
      try {
        const thought = document.createElement("em");
        thought.className = "character-thought";
        thought.setAttribute("data-editorial-role", "thought");
        range.surroundContents(thought);
        selection?.removeAllRanges();
        const selectedThought = document.createRange();
        selectedThought.selectNodeContents(thought);
        selection?.addRange(selectedThought);
      } catch {
        // Se a seleção atravessar blocos XHTML, preservamos o conteúdo e
        // aplicamos somente a ênfase nativa em vez de substituir o trecho.
        document.execCommand("italic", false);
      }
    }
    emitChange();
  };

  const applyLineHeight = (lineHeight: string) => {
    const canvas = canvasRef.current;
    const anchor = window.getSelection()?.anchorNode;
    const element = anchor?.nodeType === Node.ELEMENT_NODE ? anchor as HTMLElement : anchor?.parentElement;
    const block = element?.closest("p, h1, h2, h3, h4, h5, h6, blockquote, li") as HTMLElement | null;
    if (!canvas || !block || !canvas.contains(block)) return;
    block.style.lineHeight = lineHeight;
    emitChange();
  };

  return (
    <div className="visual-editor">
      <div className="format-toolbar" aria-label="Ferramentas de edição e leitura">
        <div className="editor-mode-switch" role="group" aria-label="Modo do livro"><button type="button" className={mode === "edit" ? "active" : ""} onClick={() => setMode("edit")}>Editar</button><button type="button" className={mode === "preview" ? "active" : ""} onClick={() => setMode("preview")}>Prévia Kindle</button></div>
        {mode === "edit" && <>
          <select aria-label="Formato do bloco" defaultValue="p" disabled={!hasDocument} onChange={(event) => changeBlock(event.target.value)}>
            <option value="p">Parágrafo</option><option value="h1">Título 1</option><option value="h2">Título 2</option><option value="h3">Título 3</option><option value="blockquote">Citação</option>
          </select>
          <span className="format-divider" />
          <button type="button" title="Negrito" disabled={!hasDocument} onMouseDown={(event) => event.preventDefault()} onClick={() => execute("bold")}><strong>B</strong></button><button type="button" title="Itálico" disabled={!hasDocument} onMouseDown={(event) => event.preventDefault()} onClick={() => execute("italic")}><em>I</em></button><button type="button" title="Sublinhado" disabled={!hasDocument} onMouseDown={(event) => event.preventDefault()} onClick={() => execute("underline")}><span className="underlined">U</span></button><button type="button" title="Tachado" disabled={!hasDocument} onMouseDown={(event) => event.preventDefault()} onClick={() => execute("strikeThrough")}><span className="struck">S</span></button>
          <span className="format-divider" />
          <button type="button" title="Alinhar à esquerda" disabled={!hasDocument} onMouseDown={(event) => event.preventDefault()} onClick={() => execute("justifyLeft")}>≡</button><button type="button" title="Centralizar" disabled={!hasDocument} onMouseDown={(event) => event.preventDefault()} onClick={() => execute("justifyCenter")}>☰</button><button type="button" title="Alinhar à direita" disabled={!hasDocument} onMouseDown={(event) => event.preventDefault()} onClick={() => execute("justifyRight")}>≡›</button><button type="button" title="Justificar" disabled={!hasDocument} onMouseDown={(event) => event.preventDefault()} onClick={() => execute("justifyFull")}>☷</button>
          <select aria-label="Espaçamento entre linhas" disabled={!hasDocument} defaultValue="" onChange={(event) => applyLineHeight(event.target.value)}><option value="" disabled>Entrelinhas</option><option value="1.3">Compacto</option><option value="1.55">Normal</option><option value="1.8">Amplo</option><option value="2">Duplo</option></select>
          <span className="format-divider" />
          <button type="button" title="Lista com marcadores" disabled={!hasDocument} onMouseDown={(event) => event.preventDefault()} onClick={() => execute("insertUnorderedList")}>• Lista</button><button type="button" title="Lista numerada" disabled={!hasDocument} onMouseDown={(event) => event.preventDefault()} onClick={() => execute("insertOrderedList")}>1. Lista</button><button type="button" title="Aumentar recuo" disabled={!hasDocument} onMouseDown={(event) => event.preventDefault()} onClick={() => execute("indent")}>›</button><button type="button" title="Diminuir recuo" disabled={!hasDocument} onMouseDown={(event) => event.preventDefault()} onClick={() => execute("outdent")}>‹</button><button type="button" title="Inserir link" disabled={!hasDocument} onMouseDown={(event) => event.preventDefault()} onClick={addLink}>↗ Link</button>
          <span className="format-divider" /><span className="quick-label">INSERIR</span><button type="button" title="Inserir travessão de diálogo" disabled={!hasDocument} onMouseDown={(event) => event.preventDefault()} onClick={() => insertQuick("dialogue")}>— Diálogo</button><button type="button" title="Inserir passagem de tempo/cena" disabled={!hasDocument} onMouseDown={(event) => event.preventDefault()} onClick={() => insertQuick("scene")}>✦ Cena</button><button type="button" title="Inserir pensamento de personagem" disabled={!hasDocument} onMouseDown={(event) => event.preventDefault()} onClick={() => insertQuick("thought")}>◌ Pensamento</button>
          <span className="format-spacer" /><button type="button" title="Desfazer" disabled={!hasDocument} onMouseDown={(event) => event.preventDefault()} onClick={() => execute("undo")}>↶</button><button type="button" title="Refazer" disabled={!hasDocument} onMouseDown={(event) => event.preventDefault()} onClick={() => execute("redo")}>↷</button>
        </>}
        {mode === "preview" && <p className="preview-mode-hint">Prévia de leitura refluída · somente leitura · a paginação acompanha fonte e tema</p>}
      </div>
      <div className="visual-editor-viewport">
        <article className={`editable-reader-page ${theme} ${mode === "preview" ? "kindle-preview" : ""}`} style={{ fontSize: `${fontScale}%`, "--canvas-brightness": String(brightness / 100), "--kindle-column-width": `${columnWidth}px` } as CSSProperties}>
          {hasDocument ? <>
            <header className="editable-running-header"><span>{displayName(documentName)}</span><span>{mode === "preview" ? "Prévia Kindle" : "Modo de edição"}</span></header>
            <div className="kindle-page-viewport">
              <div ref={canvasRef} className="xhtml-canvas" contentEditable={mode === "edit"} suppressContentEditableWarning spellCheck={mode === "edit"} onInput={emitChange} onMouseDown={(event) => { if (event.button === 0) pointerHeldRef.current = true; }} onScroll={handleCanvasScroll} aria-label={`${mode === "edit" ? "Conteúdo editável" : "Prévia de leitura"} de ${displayName(documentName)}`} />
            </div>
            <footer className="editable-page-footer"><span>Alterações refletem no XHTML ao salvar</span><span>{page} / {pageCount}</span></footer>
          </> : <div className="reader-empty"><strong>Seu livro aparece aqui</strong><p>Abra uma pasta XHTML para começar a editar visualmente.</p></div>}
        </article>
        {hasDocument && <div className="page-navigation" aria-label="Navegação de páginas">
          <button type="button" onClick={goBackward} disabled={page === 1 && !hasPreviousChapter} aria-label={page === 1 ? "Capítulo anterior" : "Página anterior"}>‹</button>
          <div><strong>Página {page} de {pageCount}</strong><span>{page === pageCount && hasNextChapter ? "Próxima seta abre o capítulo seguinte" : page === 1 && hasPreviousChapter ? "Seta anterior abre o capítulo anterior" : "Use as setas para editar e revisar página a página"}</span></div>
          <button type="button" onClick={goForward} disabled={page === pageCount && !hasNextChapter} aria-label={page === pageCount ? "Próximo capítulo" : "Próxima página"}>›</button>
        </div>}
      </div>
    </div>
  );
}
