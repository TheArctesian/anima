import type { Attachment } from 'svelte/attachments';

const FOCUSABLE = 'button:not([disabled]), [href], input:not([disabled]), [tabindex]:not([tabindex="-1"])';

function focusables(surface: HTMLElement): HTMLElement[] {
  return [...surface.querySelectorAll<HTMLElement>(FOCUSABLE)];
}

/** The element Tab should move to from `active`, wrapping at both ends; undefined when the surface has none. */
function wrapped(surface: HTMLElement, active: Element | null, backwards: boolean): HTMLElement | undefined {
  const items = focusables(surface);
  const at = active instanceof HTMLElement ? items.indexOf(active) : -1;
  if (items.length === 0) return undefined;
  if (at === -1) return backwards ? items.at(-1) : items[0];
  return items[(at + (backwards ? items.length - 1 : 1)) % items.length];
}

/**
 * Keep focus inside a modal surface while it is attached (Section 5.7.3 "Modal"): focus starts on its first
 * control (the default action), Tab and Shift+Tab wrap, a focus that lands outside comes back, and Escape calls
 * `onescape`. On detach, focus returns to where it was, or to
 * `returnTo()` when that gives an element (a caller whose opener may not have held focus, as in WebKit).
 */
export function trapFocus(onescape: () => void, returnTo?: () => HTMLElement | undefined): Attachment<HTMLElement> {
  return (surface) => {
    const before = document.activeElement;
    const keydown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        event.preventDefault();
        onescape();
      } else if (event.key === 'Tab') {
        event.preventDefault();
        wrapped(surface, document.activeElement, event.shiftKey)?.focus();
      }
    };
    const focusin = (event: FocusEvent) => {
      if (event.target instanceof Node && !surface.contains(event.target)) focusables(surface)[0]?.focus();
    };
    document.addEventListener('keydown', keydown, true);
    document.addEventListener('focusin', focusin);
    focusables(surface)[0]?.focus();
    return () => {
      document.removeEventListener('keydown', keydown, true);
      document.removeEventListener('focusin', focusin);
      const target = returnTo?.() ?? before;
      if (target instanceof HTMLElement && target.isConnected) target.focus();
    };
  };
}
