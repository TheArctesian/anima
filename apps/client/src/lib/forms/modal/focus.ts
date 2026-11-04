const FOCUSABLE = 'a[href], button, input, select, textarea, [tabindex]:not([tabindex="-1"])';

/** The elements inside `root` that Tab can reach, in document order. */
export function focusableIn(root: HTMLElement): HTMLElement[] {
  return [...root.querySelectorAll<HTMLElement>(FOCUSABLE)].filter(
    (element) => !element.hasAttribute('disabled') && !element.hidden && element.getAttribute('aria-hidden') !== 'true',
  );
}

/**
 * Where Tab must go to stay inside a dialog: the other end when it leaves the first or last element,
 * or `null` when the browser's own step stays inside.
 */
export function wrapTarget<T>(focusable: readonly T[], active: unknown, backwards: boolean): T | null {
  const first = focusable.at(0);
  const last = focusable.at(-1);
  if (first === undefined || last === undefined) return null;
  if (!focusable.some((element) => element === active)) return backwards ? last : first;
  if (backwards && active === first) return last;
  if (!backwards && active === last) return first;
  return null;
}

/** The element a dialog focuses when it opens: one marked `data-autofocus`, else the first reachable one, else the dialog. */
export function initialFocusIn(dialog: HTMLElement): HTMLElement {
  return dialog.querySelector<HTMLElement>('[data-autofocus]') ?? focusableIn(dialog).at(0) ?? dialog;
}

/** Keeps Tab inside `dialog`; returns whether it handled the event. */
export function trapTab(event: KeyboardEvent, dialog: HTMLElement): boolean {
  if (event.key !== 'Tab') return false;
  const target = wrapTarget(focusableIn(dialog), document.activeElement, event.shiftKey);
  if (target === null) return false;
  event.preventDefault();
  target.focus();
  return true;
}
