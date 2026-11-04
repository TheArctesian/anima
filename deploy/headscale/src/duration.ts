// Headscale's duration grammar for `node.expiry` (Prometheus `model.ParseDuration`):
// optional integer parts in the fixed order y w d h m s ms, at least one part, or "0".
// Anything else is rejected, as Headscale rejects it (and then silently never expires).

const second = 1000;
const minute = 60 * second;
const hour = 60 * minute;
const day = 24 * hour;

const unitMilliseconds = [365 * day, 7 * day, day, hour, minute, second, 1];

const grammar = /^(?:(\d+)y)?(?:(\d+)w)?(?:(\d+)d)?(?:(\d+)h)?(?:(\d+)m)?(?:(\d+)s)?(?:(\d+)ms)?$/;

/** Milliseconds for a Headscale duration string, or `null` if Headscale cannot parse it. */
export function parseDuration(text: string): number | null {
  if (text === '0') {
    return 0;
  }
  const match = grammar.exec(text);
  if (text === '' || match === null) {
    return null;
  }
  let total = 0;
  for (const [index, part] of match.slice(1).entries()) {
    if (part !== undefined) {
      total += Number(part) * (unitMilliseconds[index] ?? 0);
    }
  }
  return Number.isSafeInteger(total) ? total : null;
}

export const days = (count: number): number => count * day;
