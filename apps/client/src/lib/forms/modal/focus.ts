const FOCUSABLE = 'a[href], button, input:not([type="hidden"]), select, textarea, [tabindex]';

function reachable(element: HTMLElement): boolean {
  return (
    element.tabIndex >= 0 &&
    !element.matches(':disabled') &&
    element.getAttribute('aria-hidden') !== 'true' &&
    element.closest('[hidden], [inert]') === null
  );
}

/** A radio button that is not its group's Tab stop: the group is one stop, at its checked button or else its first. */
function inactiveRadio(element: HTMLElement, all: readonly HTMLElement[], active: Element | null): boolean {
  if (!(element instanceof HTMLInputElement) || element.type !== 'radio' || element.name === '') return false;
  const group = all.filter((other) => other instanceof HTMLInputElement && other.type === 'radio' && other.name === element.name);
  const stop = group.find((radio) => radio === active) ?? group.find((radio) => (radio as HTMLInputElement).checked) ?? group[0];
  return element !== stop;
}

/**
 * The elements inside `root` that Tab stops on, in document order. A hidden, inert or disabled
 * element is left out, and a group of radio buttons is one stop, as in the browser.
 */
export function focusableIn(root: HTMLElement, active: Element | null = document.activeElement): HTMLElement[] {
  const all = [...root.querySelectorAll<HTMLElement>(FOCUSABLE)].filter(reachable);
  return all.filter((element) => !inactiveRadio(element, all, active));
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
  if (event.key !== 'Tab') return false;
  event.preventDefault();
  const stops = focusableIn(dialog);
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

/** The control a click landed on, if any: the focusable element at or around its target. */
export function clickedControl(target: EventTarget | null): HTMLElement | null {
  return target instanceof Element ? target.closest<HTMLElement>(FOCUSABLE) : null;
}

/**
 * The control that opened a dialog, for focus to go back to. WebKit does not focus a button on click,
 * so `active` is then the body or a container around the button; the clicked control is taken when it
 * is inside `active` (or is it), else `active` (a dialog opened from the keyboard or by the app).
 */
export function openerOf(active: Element | null, clicked: HTMLElement | null): HTMLElement | null {
  if (clicked?.isConnected === true && (active === null || active.contains(clicked))) return clicked;
  return active instanceof HTMLElement ? active : null;
}
