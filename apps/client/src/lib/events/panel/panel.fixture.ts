import { flushSync } from 'svelte';
import type { LogEvent } from '../api';

/** The time of every test of the panel, unless it moves the clock. */
export const NOW = new Date('2026-11-04T12:00:00Z');

/** An event of the hub, `minutes` before `NOW`; `over` changes some fields. */
export function event(id: string, minutes: number, over: Partial<LogEvent> = {}): LogEvent {
  return {
    id,
    at: new Date(NOW.getTime() - minutes * 60_000).toISOString(),
    severity: 'INFO',
    source: 'CHECK',
    deviceId: 'd1',
    deviceName: 'Router',
    message: `message ${id}`,
    ...over,
  };
}

/** An `Events` answer holding `events`, newest first as the hub sends them. */
export function eventsAnswer(events: readonly LogEvent[], next: string | null = null) {
  return {
    events: {
      pageInfo: { hasNextPage: next !== null, endCursor: next },
      edges: events.map((node) => ({ node })),
    },
  };
}

/** Lets the pending promises settle and the DOM update. */
export async function settle(): Promise<void> {
  for (let round = 0; round < 5; round += 1) await Promise.resolve();
  flushSync();
}
