// Go `time.ParseDuration` grammar, for the Headscale durations read with
// `viper.GetDuration`: `node.ephemeral.inactivity_timeout`, `node.routes.ha.probe_interval`
// and `node.routes.ha.probe_timeout`. Not for `node.expiry` (see prometheus-duration.ts).
// A signed sequence of decimal numbers, each with a unit h, m, s, ms, us (µs, μs) or ns;
// "0" alone is allowed. A string without a unit is rejected here.

const nanoseconds: Record<string, number> = {
  h: 3_600_000_000_000,
  m: 60_000_000_000,
  s: 1_000_000_000,
  ms: 1_000_000,
  us: 1000,
  'µs': 1000,
  'μs': 1000,
  ns: 1,
};

const element = /(\d*(?:\.\d*)?)(ms|us|µs|μs|ns|h|m|s)/y;

/** Nanoseconds for a Go duration string, or `null` if Go cannot parse it. */
export function parseGoDuration(text: string): number | null {
  const sign = text.startsWith('-') ? -1 : 1;
  const body = /^[-+]/.test(text) ? text.slice(1) : text;
  if (body === '0') {
    return 0;
  }
  if (body === '') {
    return null;
  }
  let total = 0;
  element.lastIndex = 0;
  while (element.lastIndex < body.length) {
    const match = element.exec(body);
    const [, number = '', unit = ''] = match ?? [];
    if (match === null || !/\d/.test(number)) {
      return null;
    }
    total += Number(number) * (nanoseconds[unit] ?? Number.NaN);
  }
  return Number.isFinite(total) && total < 2 ** 63 ? sign * total : null;
}

export const seconds = (count: number): number => count * 1_000_000_000;
