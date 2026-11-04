import type { MotionValue } from 'motion';

/** One call to the fake spring engine; `frame` draws a value the way the engine would. */
export interface Run {
  readonly keyframes: readonly number[];
  readonly options: Readonly<Record<string, unknown>>;
  stopped: boolean;
  frame(value: number): void;
  finish(): void;
}

export const runs: Run[] = [];

/** Stands in for `motion`'s `animate(value, keyframes, options)` so tests control time. */
export function fakeRun(value: MotionValue<number>, keyframes: number[], options: Record<string, unknown>) {
  const { promise, resolve } = Promise.withResolvers<undefined>();
  const run: Run = {
    keyframes,
    options,
    stopped: false,
    frame: (next) => {
      value.set(next);
    },
    finish: () => {
      value.set(keyframes.at(-1) ?? value.get());
      resolve(undefined);
    },
  };
  runs.push(run);
  return {
    finished: promise,
    stop: () => {
      run.stopped = true;
    },
  };
}
