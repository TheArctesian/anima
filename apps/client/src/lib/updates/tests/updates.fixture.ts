// Test support: a scripted updater port and a log that records what it is given.
import { vi, expect } from 'vitest';
import type { AvailableUpdate, UpdateLog, UpdaterPort } from '../port';
import { CHECK_INTERVAL_MS, createUpdateSchedule, type Repeat, type UpdateSnapshot } from '../schedule';

export interface FakeUpdater extends UpdaterPort {
  checks: number;
  installs: string[];
  restarts: number;
  /** What the next `check` does; the default finds nothing. */
  next: () => Promise<AvailableUpdate | null>;
}

export function fakeUpdater(): FakeUpdater {
  const port: FakeUpdater = {
    checks: 0,
    installs: [],
    restarts: 0,
    next: () => Promise.resolve(null),
    check() {
      port.checks += 1;
      return port.next();
    },
    restart() {
      port.restarts += 1;
      return Promise.resolve();
    },
    currentVersion: () => Promise.resolve('1.0.0'),
  };
  return port;
}

/** An update that installs at once and is recorded in `port.installs`. */
export function offering(port: FakeUpdater, version: string, install?: () => Promise<void>): AvailableUpdate {
  return {
    version,
    install: () => {
      port.installs.push(version);
      return install ? install() : Promise.resolve();
    },
  };
}

export function recordingLog(): UpdateLog & { lines: string[] } {
  const lines: string[] = [];
  return {
    lines,
    warn: (text) => {
      lines.push(text);
    },
  };
}

export const START = 1_000_000;

export function scheduleHarness() {
  const port = fakeUpdater();
  const log = recordingLog();
  let clock = START;
  const history: UpdateSnapshot[] = [];
  const timers: { run: () => void; ms: number; cancelled: boolean }[] = [];
  const repeat: Repeat = (run, ms) => {
    const timer = { run, ms, cancelled: false };
    timers.push(timer);
    return () => {
      timer.cancelled = true;
    };
  };
  const schedule = createUpdateSchedule({ port, log, repeat, now: () => clock, onChange: (s) => history.push(s) });
  /** Moves the clock one interval on and fires the live timers, as the real timer does. */
  const tick = async () => {
    clock += CHECK_INTERVAL_MS;
    timers.filter((t) => !t.cancelled).forEach((t) => { t.run(); });
    await vi.waitFor(() => { expect(last().state.kind).not.toBe('checking'); });
  };
  const last = () => history.at(-1) ?? { state: { kind: 'unchecked' as const }, nextCheckAt: null };
  /** Moves the clock on without firing a timer. */
  const advance = (ms: number) => {
    clock += ms;
  };
  /** Fires the live timers at once, without waiting for the check to finish. */
  const fire = () => {
    timers.filter((t) => !t.cancelled).forEach((t) => { t.run(); });
  };
  return { port, log, schedule, timers, history, tick, last, advance, fire };
}
