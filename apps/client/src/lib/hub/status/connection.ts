import { createSubscriber } from 'svelte/reactivity';
import { INITIAL_STATUS, nextStatus, withCopyShown, type HubStatus, type StatusSignal } from './status';

/** The hub status as Svelte state: reading `current` in a component, `$derived` or `$effect` tracks it (NAV-003). */
export interface LiveStatus {
  readonly current: HubStatus;
}

/** Live status plus the `signal` input that the transport feeds; `now` stamps each signal (CQ-021). */
export function createConnection(now: () => Date): {
  status: LiveStatus;
  signal: (signal: StatusSignal) => void;
  /** The client showed its copy made at `at` in place of an answer (FND-007). */
  copyShown: (at: Date) => void;
} {
  let current = INITIAL_STATUS;
  let changed: (() => void) | undefined;
  const track = createSubscriber((update) => {
    changed = update;
    return () => {
      changed = undefined;
    };
  });
  return {
    status: {
      get current() {
        track();
        return current;
      },
    },
    signal(signal) {
      current = nextStatus(current, signal, now());
      changed?.();
    },
    copyShown(at) {
      current = withCopyShown(current, at);
      changed?.();
    },
  };
}
