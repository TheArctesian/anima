// The units of `anima agents usage`, so the page and the command read the same (crates/anima-cli/src/agents/report.rs).

/** A count as a whole number. */
export function whole(count: number): string {
  return count.toFixed(0);
}

/** A token count in readable units: `950`, `18.3k`, `2.2M`, `1.4B`. */
export function tokens(count: number): string {
  for (const [size, unit] of [
    [1e9, 'B'],
    [1e6, 'M'],
    [1e3, 'k'],
  ] as const) {
    if (count >= size) return `${(count / size).toFixed(1)}${unit}`;
  }
  return whole(count);
}

/** US dollars with cents; `<0.01` for a cost above zero that rounds to none. */
export function dollars(usd: number): string {
  return usd > 0 && usd < 0.005 ? '<0.01' : usd.toFixed(2);
}

/** The words of a cost that has no price (AIS-021). */
export const NO_PRICE = 'No price';
