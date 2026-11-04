import type { MotionValue } from 'motion';

/** One call to the fake spring engine; `frame` draws a value the way the engine would. */
export interface Run {
  readonly keyframes: readonly number[];
  readonly options: Readonly<Record<string, unknown>>;
  /** How many times the engine was asked to stop this run. */
  stops: number;
  frame(value: number): void;
  finish(): void;
}

export const runs: Run[] = [];

/** Stands in for `motion`'s `animate(value, keyframes, options)` so tests control time. */
export function fakeRun(value: MotionValue<number>, keyframes: number[], options: Record<string, unknown>) {
  let resolve: () => void = () => undefined;
  const finished = new Promise<void>((done) => {
    resolve = done;
  });
  const run: Run = {
    keyframes,
    options,
    stops: 0,
    frame: (next) => {
      value.set(next);
    },
    finish: () => {
      value.set(keyframes.at(-1) ?? value.get());
      resolve();
    },
  };
  runs.push(run);
  return {
    finished,
    stop: () => {
      run.stops += 1;
    },
  };
}
