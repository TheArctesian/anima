import type { UpdateLog, UpdaterPort } from './port';

/** The feed is checked at start and then every 4 hours (UPD-001). */
export const CHECK_INTERVAL_MS = 4 * 60 * 60 * 1000;

export const CHECK_FAILED_LINE = 'Update check failed.';
export const INSTALL_FAILED_LINE = 'Update install failed.';

export type UpdateState =
  | { readonly kind: 'idle' }
  | { readonly kind: 'checking' }
  | { readonly kind: 'downloading'; readonly version: string }
  | { readonly kind: 'installed'; readonly version: string }
  | { readonly kind: 'failed' };

/** The state, and when the next scheduled check runs (`null` before the schedule starts and once an update is installed). */
export interface UpdateSnapshot {
  readonly state: UpdateState;
  readonly nextCheckAt: number | null;
}

/** Runs `run` every `ms` until the returned function is called. Injected so tests control time. */
export type Repeat = (run: () => void, ms: number) => () => void;

export interface ScheduleDeps {
  readonly port: UpdaterPort;
  readonly log: UpdateLog;
  readonly repeat: Repeat;
  readonly now: () => number;
  readonly onChange: (snapshot: UpdateSnapshot) => void;
}

export interface UpdateSchedule {
  /** Checks now and then every 4 hours. Does nothing when already started. */
  start(): void;
  /** Stops the scheduled checks; an installed update stays installed. */
  stop(): void;
  /** One check now (UPD-006), unless one is running or an update is installed. The 4-hour cadence is not moved. */
  checkNow(): Promise<void>;
}

/**
 * The update state machine: idle, checking, downloading, installed, failed. A failure is logged once and waits for
 * the next scheduled check (UPD-002, UPD-006); once an update is installed no check runs until restart (UPD-005).
 */
export function createUpdateSchedule({ port, log, repeat, now, onChange }: ScheduleDeps): UpdateSchedule {
  let state: UpdateState = { kind: 'idle' };
  let nextCheckAt: number | null = null;
  let cancel: (() => void) | null = null;

  function set(next: UpdateState): void {
    state = next;
    onChange({ state, nextCheckAt });
  }

  function cancelTimer(): void {
    cancel?.();
    cancel = null;
    nextCheckAt = null;
  }

  function stop(): void {
    cancelTimer();
    onChange({ state, nextCheckAt });
  }

  function fail(line: string): void {
    log.warn(line);
    set({ kind: 'failed' });
  }

  async function checkNow(): Promise<void> {
    if (state.kind === 'checking' || state.kind === 'downloading' || state.kind === 'installed') return;
    set({ kind: 'checking' });
    let found;
    try {
      found = await port.check();
    } catch {
      fail(CHECK_FAILED_LINE);
      return;
    }
    if (found === null) {
      set({ kind: 'idle' });
      return;
    }
    set({ kind: 'downloading', version: found.version });
    try {
      await found.install();
    } catch {
      fail(INSTALL_FAILED_LINE);
      return;
    }
    cancelTimer();
    set({ kind: 'installed', version: found.version });
  }

  function start(): void {
    if (cancel !== null) return;
    nextCheckAt = now() + CHECK_INTERVAL_MS;
    cancel = repeat(() => {
      nextCheckAt = now() + CHECK_INTERVAL_MS;
      void checkNow();
    }, CHECK_INTERVAL_MS);
    void checkNow();
  }

  return { start, stop, checkNow };
}
