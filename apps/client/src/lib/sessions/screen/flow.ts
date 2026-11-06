/** Output the screen may hold undrawn before it starts skipping, and the level at which it draws again (bytes). */
export const HIGH_WATER_BYTES = 16 * 1024 * 1024;
export const LOW_WATER_BYTES = 4 * 1024 * 1024;

/** What to do with a piece of output: draw it, skip it, or skip it and tell the operator (the first skip of an episode). */
export type Admission = 'draw' | 'skip' | 'skip-and-tell';

/**
 * Keeps the bytes xterm.js has been given and not yet drawn under a limit (K4 §8.5; a flow-control ack is later work).
 * xterm.js throws and loses output once 50 MB wait, so above `high` new output is skipped until the backlog falls to
 * `low`; the operator is told once per episode. The session goes on: a large `cat` must not end the shell.
 */
export function outputFlow(high = HIGH_WATER_BYTES, low = LOW_WATER_BYTES) {
  let pending = 0;
  let skipping = false;
  return {
    /** Ask before writing `size` bytes; on `draw`, call `drawn(size)` when xterm.js has taken them. */
    admit: (size: number): Admission => {
      if (skipping && pending <= low) skipping = false;
      if (!skipping && pending + size > high) {
        skipping = true;
        return 'skip-and-tell';
      }
      if (skipping) return 'skip';
      pending += size;
      return 'draw';
    },
    drawn: (size: number) => {
      pending = Math.max(0, pending - size);
    },
  };
}
