import type { AccountLimit, AgentAccount, DeviceSourceRow } from '../api';
import type { Count } from '../query';
import { countOf, share } from './shape';

// What the part "By account" draws (AIS-022 as R-313 changed it): one limit for each account, shared by its devices, and
// each device's share of the program's use. Pure: the same account, rows and time give the same words.

const NUMBER_WORDS: Readonly<Record<string, number>> = { one: 1, two: 2, three: 3, four: 4, five: 5, six: 6, seven: 7 };
const MINUTES: Readonly<Record<string, number>> = { min: 1, minute: 1, hour: 60, day: 1440, week: 10_080 };

/** `count` of `unit` in words: "1 hour", "5 hours". */
function counted(count: number, unit: string): string {
  return `${String(count)} ${unit}${count === 1 ? '' : 's'}`;
}

/** A length in minutes in the largest whole unit: 300 is "5 hours", 10080 is "7 days". */
function lengthWords(minutes: number): string {
  if (minutes % 1440 === 0) return counted(minutes / 1440, 'day');
  if (minutes % 60 === 0) return counted(minutes / 60, 'hour');
  return counted(minutes, 'minute');
}

/** The name of a limit window in words: `five_hour` and `300 min` are "5 hours", `seven_day` and `weekly` are "7 days". */
export function windowWords(window: string): string {
  if (window === 'weekly') return lengthWords(MINUTES.week ?? 0);
  if (window === 'daily') return lengthWords(MINUTES.day ?? 0);
  const match = /^(\d+|[a-z]+)[_ ](min|minute|hour|day|week)s?$/.exec(window);
  const amount = match === null ? undefined : (NUMBER_WORDS[match[1] ?? ''] ?? Number(match[1]));
  const unit = match === null ? undefined : MINUTES[match[2] ?? ''];
  if (amount === undefined || unit === undefined || !Number.isInteger(amount) || amount <= 0) {
    const words = window.replaceAll('_', ' ');
    return words.charAt(0).toUpperCase() + words.slice(1);
  }
  return lengthWords(amount * unit);
}

/** How full a window is: `reached` at its limit, `high` from 80 % used, else `normal`. */
export function limitLevel(limit: Pick<AccountLimit, 'usedPercent' | 'reached'>): 'reached' | 'high' | 'normal' {
  if (limit.reached) return 'reached';
  return (limit.usedPercent ?? 0) >= 80 ? 'high' : 'normal';
}

/** The width of the bar of a window, 0 to 100: a reached window is full. */
export function limitFill(limit: Pick<AccountLimit, 'usedPercent' | 'reached'>): number {
  return limit.reached ? 100 : Math.min(100, Math.max(0, limit.usedPercent ?? 0));
}

const DAY_MS = 86_400_000;

/** When a window starts again, in the time of `zone`: "14:00" today, "Mon 09:00" in the next six days, else "28 Nov". */
export function resetWords(resetsAt: string, now: number, zone?: string): string {
  const at = Date.parse(resetsAt);
  const day = (time: number) => new Intl.DateTimeFormat('en-CA', { timeZone: zone, dateStyle: 'short' }).format(time);
  const clock = new Intl.DateTimeFormat('en-GB', { timeZone: zone, hour: '2-digit', minute: '2-digit', hourCycle: 'h23' }).format(at);
  if (day(at) === day(now)) return clock;
  if (at - now < 6 * DAY_MS) return `${new Intl.DateTimeFormat('en-GB', { timeZone: zone, weekday: 'short' }).format(at)} ${clock}`;
  return new Intl.DateTimeFormat('en-GB', { timeZone: zone, day: 'numeric', month: 'short' }).format(at);
}

/** The line beside a window: "35 % used", "Limit reached", with "starts again" and its time when the program said it. */
export function limitText(limit: AccountLimit, now: number, zone?: string): string {
  const used = limit.reached ? 'Limit reached' : limit.usedPercent === null ? 'Not reached' : `${String(limit.usedPercent)} % used`;
  return limit.resetsAt === null ? used : `${used} · starts again ${resetWords(limit.resetsAt, now, zone)}`;
}

/** One device of an account with its share of the program's use in the days of the page. */
export interface DeviceShare {
  readonly id: string;
  readonly name: string;
  /** 0 to 100; null when no device of the account has use of the program in these days. */
  readonly percent: number | null;
}

/** Each device of `account` with its share of what the page counts, for the program of the account only. */
export function deviceShares(account: Pick<AgentAccount, 'source' | 'devices'>, rows: readonly DeviceSourceRow[], count: Count): DeviceShare[] {
  const counts = account.devices.map((device) => {
    const row = rows.find((one) => one.device.id === device.id && one.source === account.source);
    return row === undefined ? 0 : countOf(row, count);
  });
  const sum = counts.reduce((all, one) => all + one, 0);
  return account.devices.map((device, index) => ({
    id: device.id,
    name: device.name,
    percent: sum === 0 ? null : Math.round(share(counts[index] ?? 0, sum) * 100),
  }));
}
