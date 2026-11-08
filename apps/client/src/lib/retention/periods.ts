import type { RetentionPeriod, RetentionSubject } from './api';

/** The kinds of data the hub keeps for a limited time, in the order Settings lists them, with the operator's name for each (D-25). */
export const SUBJECTS: readonly { readonly subject: RetentionSubject; readonly label: string }[] = [
  { subject: 'HEARTBEATS', label: 'Check results' },
  { subject: 'METRICS', label: 'Metrics' },
  { subject: 'EVENTS', label: 'Events' },
  { subject: 'RECORDINGS', label: 'Session recordings' },
  { subject: 'DELIVERIES', label: 'Notification deliveries' },
];

export type DaysCheck = { readonly ok: true; readonly days: number } | { readonly ok: false; readonly message: string };

/** The days the operator typed, or the message that says what to change. Nothing is sent when this refuses (HUB-005). */
export function checkDays(text: string, period: Pick<RetentionPeriod, 'minDays' | 'maxDays'>): DaysCheck {
  const trimmed = text.trim();
  if (!/^-?\d+$/.test(trimmed)) return { ok: false, message: 'Enter a whole number of days.' };
  const days = Number(trimmed);
  if (days < period.minDays || days > period.maxDays) {
    return { ok: false, message: `Enter a number of days from ${String(period.minDays)} to ${String(period.maxDays)}.` };
  }
  return { ok: true, days };
}

/** The periods the page knows, in page order; a subject the hub did not list has no row. */
export function inPageOrder(periods: readonly RetentionPeriod[]): readonly RetentionPeriod[] {
  return SUBJECTS.flatMap(({ subject }) => periods.filter((period) => period.subject === subject));
}

export function daysText(days: number): string {
  return days === 1 ? '1 day' : `${String(days)} days`;
}
