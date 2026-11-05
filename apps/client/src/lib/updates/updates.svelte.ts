import { createContext } from 'svelte';
import type { UpdaterPort } from '$lib/desktop/updater';
import { createUpdateSchedule, type Repeat, type UpdateLog, type UpdateSnapshot } from './schedule';

const everyInterval: Repeat = (run, ms) => {
  const timer = setInterval(run, ms);
  return () => {
    clearInterval(timer);
  };
};

/** The update schedule as Svelte state for the banner and the Settings row. */
export class Updates {
  snapshot = $state<UpdateSnapshot>({ state: { kind: 'unchecked' }, nextCheckAt: null });
  /** The running app's version, once `loadVersion` has read it. */
  current = $state<string | null>(null);

  readonly #port: UpdaterPort;
  readonly #schedule: ReturnType<typeof createUpdateSchedule>;

  constructor(port: UpdaterPort, log: UpdateLog, repeat: Repeat = everyInterval, now: () => number = Date.now) {
    this.#port = port;
    this.#schedule = createUpdateSchedule({
      port,
      log,
      repeat,
      now,
      onChange: (next) => {
        this.snapshot = next;
      },
    });
  }

  /** Starts the scheduled checks (UPD-001); the returned function stops them. */
  start(): () => void {
    this.#schedule.start();
    return () => {
      this.#schedule.stop();
    };
  }

  checkNow(): Promise<void> {
    return this.#schedule.checkNow();
  }

  /** Restarts into the installed update; call only from the operator's choice (UPD-003). */
  restart(): Promise<void> {
    return this.#port.restart();
  }

  /** Reads the running version; it stays `null` when the shell cannot say. */
  async loadVersion(): Promise<void> {
    try {
      this.current = await this.#port.currentVersion();
    } catch {
      this.current = null;
    }
  }
}

const [getUpdates, setUpdates] = createContext<Updates>();

/** Make `updates` available to every component below; call once in the root layout. */
export function provideUpdates(updates: Updates): void {
  setUpdates(updates);
}

/** The updates set by `provideUpdates` in a parent component; throws when there is none. */
export function useUpdates(): Updates {
  return getUpdates();
}
