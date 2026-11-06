export const FOCUSABLE = 'a[href], button, input:not([type="hidden"]), select, textarea, [tabindex]';

function reachable(element: HTMLElement): boolean {
  return (
    element.tabIndex >= 0 &&
    !element.matches(':disabled') &&
    element.getAttribute('aria-hidden') !== 'true' &&
    element.closest('[hidden], [inert]') === null
  );
}

function radioGroup(element: HTMLElement, all: readonly HTMLElement[]): HTMLInputElement[] | null {
  if (!(element instanceof HTMLInputElement) || element.type !== 'radio' || element.name === '') return null;
  return all.filter(
    (other): other is HTMLInputElement =>
      other instanceof HTMLInputElement && other.type === 'radio' && other.name === element.name && other.form === element.form,
  );
}

/**
 * A radio button that is not its group's Tab stop. The group is one stop: the focused button, else the
 * checked one, else the first (the last when Tab goes backwards), as in the browser.
 */
function inactiveRadio(element: HTMLElement, all: readonly HTMLElement[], active: Element | null, backwards: boolean): boolean {
  const group = radioGroup(element, all);
  if (group === null) return false;
  const edge = backwards ? group.at(-1) : group[0];
  const stop = group.find((radio) => radio === active) ?? group.find((radio) => radio.checked) ?? edge;
  return element !== stop;
}

/**
 * The elements inside `root` that Tab stops on, in document order. A hidden, inert or disabled
 * element is left out, and a group of radio buttons is one stop, as in the browser.
 */
export function focusableIn(root: HTMLElement, active: Element | null = document.activeElement, backwards = false): HTMLElement[] {
  const all = [...root.querySelectorAll<HTMLElement>(FOCUSABLE)].filter(reachable);
  return all.filter((element) => !inactiveRadio(element, all, active, backwards));
}

/**
 * Where Tab goes from `active` among a dialog's stops: the next one (the previous with Shift),
 * round from the last to the first; from outside the stops, the first (the last with Shift).
 */
export function tabTarget<T>(stops: readonly T[], active: unknown, backwards: boolean): T | null {
  if (stops.length === 0) return null;
  const at = stops.findIndex((stop) => stop === active);
  if (at === -1) return backwards ? (stops.at(-1) ?? null) : (stops[0] ?? null);
  return stops[(at + (backwards ? stops.length - 1 : 1)) % stops.length] ?? null;
}

/** The element a dialog focuses when it opens: one marked `data-autofocus`, else the first reachable one, else the dialog. */
export function initialFocusIn(dialog: HTMLElement): HTMLElement {
  return dialog.querySelector<HTMLElement>('[data-autofocus]') ?? focusableIn(dialog).at(0) ?? dialog;
}

/**
 * Keeps Tab inside `dialog` and moves the focus itself, so every stop is reached in every engine:
 * WebKit's own Tab skips buttons and links and can leave the dialog. A stop that does not take the
 * focus (one not rendered) is passed over. Returns whether it handled the event.
 */
export function trapTab(event: KeyboardEvent, dialog: HTMLElement): boolean {
  if (event.key !== 'Tab' || event.ctrlKey || event.altKey || event.metaKey) return false;
  event.preventDefault();
  const stops = focusableIn(dialog, document.activeElement, event.shiftKey);
  const tried = new Set<HTMLElement>();
  let target = tabTarget(stops, document.activeElement, event.shiftKey);
  while (target !== null && !tried.has(target)) {
    tried.add(target);
    target.focus();
    if (document.activeElement === target) return true;
    target = tabTarget(stops, target, event.shiftKey);
  }
  dialog.focus();
  return true;
}
