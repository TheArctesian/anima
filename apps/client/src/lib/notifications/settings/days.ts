// The certificate expiry thresholds as the operator types them, "21, 14, 7", checked against D-22 before sending:
// whole days from 1 to 365, at most five, largest first and each once (NOT-006).
const DAYS_MIN = 1;
const DAYS_MAX = 365;
const COUNT_MAX = 5;
export const DEFAULT_DAYS: readonly number[] = [21, 14, 7];

export const DAYS_MISSING = 'Enter at least one number of days.';
export const DAYS_NOT_WHOLE = 'Use whole days, separated by commas.';
export const DAYS_OUT_OF_RANGE = `Use days from ${String(DAYS_MIN)} to ${String(DAYS_MAX)}.`;
export const DAYS_TOO_MANY = `Use at most ${String(COUNT_MAX)} values.`;
export const DAYS_NOT_DESCENDING = 'List the largest first, each once.';

type ThresholdsCheck = { readonly ok: true; readonly days: number[] } | { readonly ok: false; readonly message: string };

/** Commas, spaces or both separate the values. */
function parts(text: string): string[] {
  return text.split(/[\s,]+/).filter((part) => part !== '');
}

function descending(days: readonly number[]): boolean {
  return days.every((day, index) => index === 0 || day < (days[index - 1] ?? Number.POSITIVE_INFINITY));
}

export function checkThresholds(text: string): ThresholdsCheck {
  const found = parts(text);
  if (found.length === 0) return { ok: false, message: DAYS_MISSING };
  if (!found.every((part) => /^\d{1,4}$/.test(part))) return { ok: false, message: DAYS_NOT_WHOLE };
  const days = found.map(Number);
  if (days.some((day) => day < DAYS_MIN || day > DAYS_MAX)) return { ok: false, message: DAYS_OUT_OF_RANGE };
  if (days.length > COUNT_MAX) return { ok: false, message: DAYS_TOO_MANY };
  return descending(days) ? { ok: true, days } : { ok: false, message: DAYS_NOT_DESCENDING };
}

export function thresholdsText(days: readonly number[]): string {
  return days.join(', ');
}
