import type { LoadingClock } from './loading';

/** A clock a test moves by hand: `advance` runs what has come due. */
export function manualClock() {
  let time = 0;
  let fired = 0;
  const waiting: { at: number; run: () => void; live: boolean }[] = [];
  const clock: LoadingClock = {
    now: () => time,
    after: (ms, run) => {
      const entry = { at: time + ms, run, live: true };
      waiting.push(entry);
      return () => {
        entry.live = false;
      };
    },
  };
  return {
    clock,
    /** Timers set and not yet fired or cancelled. */
    pending: (): number => waiting.filter((entry) => entry.live).length,
    /** Timers that have fired. */
    fired: (): number => fired,
    advance: (ms: number): void => {
      const target = time + ms;
      for (;;) {
        const due = waiting.filter((entry) => entry.live && entry.at <= target).toSorted((a, b) => a.at - b.at)[0];
        if (due === undefined) break;
        due.live = false;
        time = due.at;
        fired += 1;
        due.run();
      }
      time = target;
    },
  };
}
