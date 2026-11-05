import { flushSync } from 'svelte';
import { fakeHub } from '$lib/devices/hub.fixture';
import type { Hub, HubResult, Variables } from '$lib/hub/hub';
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

/** An `Events` request that has not been answered yet. */
export interface Pending {
  readonly variables: Variables;
  /** Answer it now, so a test controls the order of answers. */
  readonly answer: (result: HubResult<unknown>) => void;
}

/** A fake hub whose `Events` requests wait in `pending` until a test answers them; everything else is as `fakeHub`. */
export function deferredHub() {
  const fake = fakeHub();
  const pending: Pending[] = [];
  const hub: Hub = {
    ...fake.hub,
    request: (document, variables) => {
      const isEvents = document.definitions.some(
        (part) => part.kind === 'OperationDefinition' && part.name?.value === 'Events',
      );
      if (!isEvents) return fake.hub.request(document, variables);
      return new Promise((resolve) => {
        pending.push({
          variables,
          answer: (result) => {
            resolve(result as never);
          },
        });
      });
    },
  };
  return { ...fake, hub, pending };
}

/** The successful answer to a pending `Events` request. */
export const page = (events: readonly LogEvent[], next: string | null = null): HubResult<unknown> => ({
  ok: true,
  value: eventsAnswer(events, next),
});
