import { FOCUSABLE } from './focus';

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

/** The control of the latest click; a key press forgets it, so a dialog opened from the keyboard never takes an old click. */
let clicked: HTMLElement | null = null;
let watchers = 0;

const record = (event: MouseEvent) => {
  clicked = clickedControl(event.target);
};
const forget = () => {
  clicked = null;
};

/**
 * Watches clicks on the document while at least one dialog is mounted; one pair of listeners serves
 * every dialog. It must run while the dialog is closed, because the click that opens it comes first.
 * Returns the function that stops this dialog's watch.
 */
export function watchClicks(): () => void {
  if (watchers === 0) {
    document.addEventListener('click', record, { capture: true });
    document.addEventListener('keydown', forget, { capture: true });
  }
  watchers += 1;
  return () => {
    watchers -= 1;
    if (watchers > 0) return;
    document.removeEventListener('click', record, { capture: true });
    document.removeEventListener('keydown', forget, { capture: true });
    clicked = null;
  };
}

/** The opener for a dialog that opens now; the click is used once. */
export function takeOpener(active: Element | null): HTMLElement | null {
  const opener = openerOf(active, clicked);
  clicked = null;
  return opener;
}

/** The dialogs that are open, oldest first: only the last one answers keys pressed outside every dialog. */
const open: HTMLElement[] = [];

/** Records `dialog` as open and returns the function that removes it. */
export function enlist(dialog: HTMLElement): () => void {
  open.push(dialog);
  return () => {
    const at = open.indexOf(dialog);
    if (at !== -1) open.splice(at, 1);
  };
}

export function isTopmost(dialog: HTMLElement): boolean {
  return open.at(-1) === dialog;
}
