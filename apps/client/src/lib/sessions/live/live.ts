import type { LiveSession } from './api';

/** The control's name: the count is part of it, so a screen reader hears it (RA-034). */
export function liveLabel(count: number): string {
  return `Live sessions, ${String(count)}`;
}

/** The time a session started, in the operator's locale. */
export function startedTime(session: LiveSession): string {
  return new Date(session.startedAt).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
}

/** Where Tab or Shift+Tab goes from the open popover (C-3): the Stop buttons form a ring, and the control is before the first. */
export type TabStop = { readonly in: number } | { readonly to: 'control' };

/**
 * The next stop for Tab (or Shift+Tab with `back`) when focus is on `at`: -1 is the control, 0 and up are the
 * Stop buttons of `count`. Tab from the last Stop wraps to the first; Shift+Tab from the first goes to the control.
 */
export function nextTabStop(at: number, count: number, back: boolean): TabStop {
  if (count === 0) return { to: 'control' };
  if (back) return at <= 0 ? { to: 'control' } : { in: at - 1 };
  return { in: at + 1 >= count ? 0 : at + 1 };
}
