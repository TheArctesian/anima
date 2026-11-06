import { timeLabel } from '$lib/events/panel/view';
import type { SessionEndReason } from '$lib/hub/generated/graphql';
import type { SessionPage } from '$lib/sessions/api';

/** One past or live session of the device (Section 6.7). */
export type DeviceSession = SessionPage['edges'][number]['node'];

/** A session as the Sessions tab shows it (D-10). */
export interface SessionRow {
  readonly id: string;
  readonly start: string;
  readonly end: string;
  readonly duration: string;
  readonly operator: string;
  readonly reason: string;
}

const REASONS: Record<SessionEndReason, string> = {
  OPERATOR: 'Closed by the operator',
  REMOTE_CLOSED: 'Closed by the device',
  GATEWAY_ERROR: 'Gateway error',
  AUTH_FAILED: 'Authentication failed',
  HOST_KEY_MISMATCH: 'Host key changed',
  PROTOCOL_ERROR: 'Protocol error',
  TIMEOUT: 'Timed out',
  HUB_SHUTDOWN: 'Hub stopped',
};

const MINUTE_S = 60;
const HOUR_S = 3600;

/** A length of time in seconds as "1 h 5 min", "4 min 2 s" or "12 s". */
export function durationText(seconds: number): string {
  const whole = Math.max(0, Math.floor(seconds));
  const hours = Math.floor(whole / HOUR_S);
  const minutes = Math.floor((whole % HOUR_S) / MINUTE_S);
  const rest = whole % MINUTE_S;
  if (hours > 0) return `${String(hours)} h ${String(minutes)} min`;
  if (minutes > 0) return `${String(minutes)} min ${String(rest)} s`;
  return `${String(rest)} s`;
}

function row(session: DeviceSession, now: Date): SessionRow {
  const started = Date.parse(session.startedAt);
  const ended = session.endedAt == null ? now.getTime() : Date.parse(session.endedAt);
  const live = session.endedAt == null;
  return {
    id: session.id,
    start: timeLabel(session.startedAt, now),
    end: live ? 'Live' : timeLabel(session.endedAt ?? '', now),
    duration: Number.isNaN(started) || Number.isNaN(ended) ? '—' : durationText((ended - started) / 1000),
    operator: session.operatorName,
    reason: session.endReason == null ? '—' : REASONS[session.endReason],
  };
}

/** D-10: the rows newest first by start; a live session's duration runs to `now`. */
export function sessionRows(sessions: readonly DeviceSession[], now: Date): readonly SessionRow[] {
  return sessions.toSorted((a, b) => Date.parse(b.startedAt) - Date.parse(a.startedAt)).map((session) => row(session, now));
}
