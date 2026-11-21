import { formatBytes } from '$lib/metrics/units';

/**
 * The figures of the account for the month of now, as the pool gives them (HMD-073 in form B, task HM-T38 of
 * hosted-mode.md; the form is `contracts/account-usage.md`). The hub holds none of them: the page asks the account.
 */
export interface AccountUsage {
  /** When the pool computed the figures. */
  readonly at: Date;
  /** The month, `YYYY-MM`. */
  readonly month: string;
  readonly plan: { readonly name: string; readonly charged: boolean };
  readonly awakeSeconds: number;
  readonly asleepSeconds: number;
  /** The amount so far, before tax, in minor units of `currency`; `null` on a plan that is not charged. */
  readonly amount: { readonly minor: number; readonly currency: string } | null;
  /** The account owes for an earlier month: the page points to the account page and shows no sum of it. */
  readonly owes: boolean;
  /** What the quota of the plan counts: the limit, and the used bytes when the pool can read them. */
  readonly disk: { readonly limitBytes: number; readonly usedBytes: number | null };
}

/** What the page shows in the place of the figures of the account. */
export type UsageAnswer =
  | { readonly state: 'figures'; readonly usage: AccountUsage }
  | { readonly state: 'signed-out' }
  | { readonly state: 'unavailable' };

/** The account page, where an amount that is owed is shown and paid. */
export const ACCOUNT_PAGE = 'https://cloud.anima.ing/account';

export const ESTIMATE_NOTE = 'So far this month, before tax. This is an estimate, not a bill. Your invoice arrives after the month ends.';
export const NOT_CHARGED = 'Nothing is charged on this plan';
export const NOT_AVAILABLE = 'Not available just now';
export const SIGNED_OUT = 'Sign in to your Anima.ing account to see the figures of your plan.';
export const OWES = 'Your account has an amount due from an earlier month.';

const isRecord = (value: unknown): value is Record<string, unknown> => typeof value === 'object' && value !== null;
const isCount = (value: unknown): value is number => typeof value === 'number' && Number.isSafeInteger(value) && value >= 0;

function plan(value: unknown): AccountUsage['plan'] | undefined {
  if (!isRecord(value) || typeof value.name !== 'string' || value.name === '' || typeof value.charged !== 'boolean') return undefined;
  return { name: value.name, charged: value.charged };
}

function amount(value: unknown): AccountUsage['amount'] | undefined {
  if (value === null) return null;
  if (!isRecord(value) || !Number.isSafeInteger(value.minor) || typeof value.currency !== 'string') return undefined;
  return /^[A-Z]{3}$/.test(value.currency) ? { minor: value.minor as number, currency: value.currency } : undefined;
}

function disk(value: unknown): AccountUsage['disk'] | undefined {
  if (!isRecord(value) || !isCount(value.limitBytes)) return undefined;
  if (value.usedBytes !== null && !isCount(value.usedBytes)) return undefined;
  return { limitBytes: value.limitBytes, usedBytes: value.usedBytes };
}

/** The answer of the pool when it has the form of the contract; `undefined` when any member is missing or wrong. */
export function parseUsage(value: unknown): AccountUsage | undefined {
  if (!isRecord(value) || typeof value.at !== 'string' || typeof value.month !== 'string' || !/^\d{4}-\d{2}$/.test(value.month)) return undefined;
  const at = new Date(value.at);
  const [shownPlan, shownAmount, shownDisk] = [plan(value.plan), amount(value.amount), disk(value.disk)];
  if (Number.isNaN(at.getTime()) || shownPlan === undefined || shownAmount === undefined || shownDisk === undefined) return undefined;
  if (!isCount(value.awakeSeconds) || !isCount(value.asleepSeconds) || typeof value.owes !== 'boolean') return undefined;
  return { at, month: value.month, plan: shownPlan, awakeSeconds: value.awakeSeconds, asleepSeconds: value.asleepSeconds, amount: shownAmount, owes: value.owes, disk: shownDisk };
}

/** Hours, rounded down: one decimal under 10 hours, whole hours from 10 (`0.5 h`, `12 h`). */
export function hoursText(seconds: number): string {
  const hours = seconds / 3600;
  return hours < 10 ? `${(Math.floor(hours * 10) / 10).toString()} h` : `${Math.floor(hours).toString()} h`;
}

/** The amount in its currency, with the digits the currency has (`$4.12`, `¥412`). */
export function amountText(value: NonNullable<AccountUsage['amount']>, locale?: string): string {
  const format = new Intl.NumberFormat(locale, { style: 'currency', currency: value.currency });
  const digits = format.resolvedOptions().maximumFractionDigits ?? 2;
  return format.format(value.minor / 10 ** digits);
}

/** "Disk used of the plan": the used bytes against the limit, with a share only when the pool gave the used bytes. */
export function planDiskText(value: AccountUsage['disk']): string {
  const limit = formatBytes(value.limitBytes);
  if (value.usedBytes === null) return `Limit ${limit}`;
  const share = value.limitBytes === 0 ? 0 : Math.round((value.usedBytes / value.limitBytes) * 100);
  return `${formatBytes(value.usedBytes)} of ${limit} (${share.toString()}%)`;
}

/** When the figures were computed, as the page says it: `just now`, `12 min ago`, or the time. */
export function figuresTime(at: Date, now: Date): string {
  const minutes = Math.floor((now.getTime() - at.getTime()) / 60_000);
  if (minutes < 1) return 'Figures from just now';
  if (minutes < 60) return `Figures from ${minutes.toString()} min ago`;
  return `Figures from ${at.toLocaleString(undefined, { dateStyle: 'medium', timeStyle: 'short' })}`;
}

/** The newest bytes of the hub's data: the last `DISK_BYTES` value of the hub's own device (HMD-072). */
export function hubDataBytes(series: readonly { readonly kind: string; readonly points: readonly { readonly value: number | null }[] }[]): number | null {
  const bytes = series.find((one) => one.kind === 'DISK_BYTES');
  const values = (bytes?.points ?? []).map((point) => point.value).filter((value): value is number => value !== null);
  return values.at(-1) ?? null;
}
