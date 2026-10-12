import { useEffect, useRef, type RefObject } from "react";

// Pilha de diálogos abertos: só o do topo reage a Tab/Esc (um recap dentro do caderno, por exemplo).
const openDialogs: HTMLElement[] = [];

const FOCUSABLE = 'a[href], summary, button:not([disabled]), input:not([disabled]):not([type="hidden"]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])';

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
      const visible = (node: HTMLElement) => (typeof node.checkVisibility === "function" ? node.checkVisibility({ visibilityProperty: true, contentVisibilityAuto: true }) : node.offsetParent !== null);
      const all = [...dialog.querySelectorAll<HTMLElement>(FOCUSABLE)].filter((node) => visible(node) && !node.closest("[inert]"));
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
      // O ciclo é feito à mão: o Chrome também põe áreas roláveis na ordem do Tab e o foco escaparia do diálogo.
      const current = nodes.indexOf(document.activeElement as HTMLElement);
      const next = event.shiftKey ? (current <= 0 ? nodes.length - 1 : current - 1) : (current === -1 || current === nodes.length - 1 ? 0 : current + 1);
      event.preventDefault();
      // Se um elemento recusar o foco (ex.: dentro de <details> fechado), tenta o seguinte.
      for (let step = 0; step < nodes.length; step += 1) {
        const target = nodes[(next + (event.shiftKey ? -step : step) + nodes.length * 2) % nodes.length];
        target.focus();
        if (document.activeElement === target) break;
      }
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
