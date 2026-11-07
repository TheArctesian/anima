import type { LiveSession } from './api';

/** The control's name: the count is part of it, so a screen reader hears it (RA-034). */
export function liveLabel(count: number): string {
  return `Live sessions, ${String(count)}`;
}

/** The time a session started, in the operator's locale. */
export function startedTime(session: LiveSession): string {
  return new Date(session.startedAt).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
}
