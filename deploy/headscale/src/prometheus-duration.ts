// Headscale's grammar for `node.expiry` only (Prometheus `model.ParseDuration`):
// optional integer parts in the fixed order y w d h m s ms, at least one part, or "0".
// The total must fit a Go `time.Duration` (int64 nanoseconds, about 292 years).
// Anything else is rejected, as Headscale rejects it (and then silently never expires).
// Other Headscale durations use Go's grammar: see go-duration.ts.

const second = 1000;
const minute = 60 * second;
const hour = 60 * minute;
const day = 24 * hour;

const unitMilliseconds = [365 * day, 7 * day, day, hour, minute, second, 1] as const;

// floor((2^63 - 1) ns / 1e6): the largest whole number of milliseconds Go can hold.
const maxMilliseconds = 9_223_372_036_854;

const grammar = /^(?:(\d+)y)?(?:(\d+)w)?(?:(\d+)d)?(?:(\d+)h)?(?:(\d+)m)?(?:(\d+)s)?(?:(\d+)ms)?$/;

/** Milliseconds for a `node.expiry` string, or `null` if Headscale cannot parse it. */
export function parsePrometheusDuration(text: string): number | null {
  if (text === '0') {
    return 0;
  }
  const match = grammar.exec(text);
  if (text === '' || match === null) {
    return null;
  }
  let total = 0;
  unitMilliseconds.forEach((unit, index) => {
    const part = match[index + 1];
    if (part !== undefined) {
      total += Number(part) * unit;
    }
  });
  return total <= maxMilliseconds ? total : null;
}

export const days = (count: number): number => count * day;
