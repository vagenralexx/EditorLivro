import { useEffect, useRef, type RefObject } from "react";

// Pilha de diálogos abertos: só o do topo reage a Tab/Esc (um recap dentro do caderno, por exemplo).
const openDialogs: HTMLElement[] = [];

const FOCUSABLE = 'a[href], button:not([disabled]), input:not([disabled]):not([type="hidden"]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])';

/**
 * Acessibilidade de diálogo modal: foco inicial dentro dele, Tab preso no diálogo,
 * Esc fecha (chamando o mesmo manipulador do botão ×) e o foco volta ao elemento que o abriu.
 */
export function useDialogA11y<T extends HTMLElement>(onClose: () => void): RefObject<T | null> {
  const ref = useRef<T>(null);
  const closeRef = useRef(onClose);
  closeRef.current = onClose;

  useEffect(() => {
    const dialog = ref.current;
    if (!dialog) return undefined;
    const previous = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    const previousTag = previous?.tagName;
    const previousLabel = previous?.textContent?.trim() ?? "";
    openDialogs.push(dialog);
    // Radios de um mesmo grupo são UMA parada de Tab (a marcada, ou a primeira se nenhuma está marcada).
    const focusables = () => {
      const all = [...dialog.querySelectorAll<HTMLElement>(FOCUSABLE)].filter((node) => node.offsetParent !== null && !node.closest("[inert]"));
      return all.filter((node) => {
        if (!(node instanceof HTMLInputElement) || node.type !== "radio" || !node.name) return true;
        const group = all.filter((other): other is HTMLInputElement => other instanceof HTMLInputElement && other.type === "radio" && other.name === node.name);
        return (group.find((radio) => radio.checked) ?? group[0]) === node;
      });
    };
    if (!dialog.contains(document.activeElement)) (dialog.querySelector<HTMLElement>("[autofocus]") ?? focusables()[0] ?? dialog).focus();

    const onKeyDown = (event: KeyboardEvent) => {
      if (openDialogs[openDialogs.length - 1] !== dialog) return;
      if (event.key === "Escape" && !event.defaultPrevented) { event.preventDefault(); event.stopPropagation(); closeRef.current(); return; }
      if (event.key !== "Tab") return;
      const nodes = focusables();
      if (nodes.length === 0) { event.preventDefault(); dialog.focus(); return; }
      const [first, last] = [nodes[0], nodes[nodes.length - 1]];
      if (event.shiftKey && (document.activeElement === first || !dialog.contains(document.activeElement))) { event.preventDefault(); last.focus(); }
      else if (!event.shiftKey && (document.activeElement === last || !dialog.contains(document.activeElement))) { event.preventDefault(); first.focus(); }
    };
    document.addEventListener("keydown", onKeyDown, true);
    return () => {
      document.removeEventListener("keydown", onKeyDown, true);
      const at = openDialogs.indexOf(dialog);
      if (at >= 0) openDialogs.splice(at, 1);
      // O botão que abriu o diálogo pode ter sido redesenhado: nesse caso procura o equivalente pelo texto.
      if (previous && document.contains(previous)) previous.focus();
      else if (previousTag && previousLabel) [...document.querySelectorAll<HTMLElement>(previousTag)].find((node) => node.textContent?.trim() === previousLabel && node.offsetParent !== null)?.focus();
    };
  }, []);

  return ref;
}
