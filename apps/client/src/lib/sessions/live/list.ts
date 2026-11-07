import type { LiveSession, SessionChange } from './api';

/** The sessions, newest start first; equal starts keep the order they came in. */
export function newestFirst(sessions: readonly LiveSession[]): readonly LiveSession[] {
  return sessions.toSorted((a, b) => Date.parse(b.startedAt) - Date.parse(a.startedAt));
}

/** The list without session `id`; an unknown `id` changes nothing. */
export function withoutSession(sessions: readonly LiveSession[], id: string): readonly LiveSession[] {
  return sessions.some((session) => session.id === id) ? sessions.filter((session) => session.id !== id) : sessions;
}

/** Apply one change: a running session is added or replaced, an ended one is removed (RA-034, D-6). */
export function applyChange(sessions: readonly LiveSession[], change: SessionChange): readonly LiveSession[] {
  if (change.endedAt !== null) return withoutSession(sessions, change.id);
  const { id, deviceId, deviceName, protocol, operatorName, startedAt, clientAddr } = change;
  const running: LiveSession = { id, deviceId, deviceName, protocol, operatorName, startedAt, clientAddr };
  return newestFirst([...sessions.filter((session) => session.id !== id), running]);
}
