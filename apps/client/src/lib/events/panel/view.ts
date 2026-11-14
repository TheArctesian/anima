import type { EventSource, Severity } from '$lib/hub/generated/graphql';
import { toggled } from '$lib/devices/toolbar/toolbar';
import type { IconName } from '$lib/icons/icons';
import type { LogFilter, Since } from '../log';

export interface Choice<T extends string> {
  readonly value: T;
  readonly label: string;
}

export const SINCE_CHOICES: readonly Choice<Since>[] = [
  { value: '1h', label: '1 h' },
  { value: '24h', label: '24 h' },
  { value: '7d', label: '7 d' },
  { value: 'all', label: 'All' },
];

export const SEVERITY_CHOICES: readonly Choice<Severity>[] = [
  { value: 'ERROR', label: 'Error' },
  { value: 'WARNING', label: 'Warning' },
  { value: 'INFO', label: 'Info' },
];

export const SOURCE_CHOICES: readonly Choice<EventSource>[] = [
  { value: 'CHECK', label: 'Check' },
  { value: 'SESSION', label: 'Session' },
  { value: 'DEVICE', label: 'Device' },
  { value: 'ACCESS', label: 'Access' },
  { value: 'HUB', label: 'Hub' },
  { value: 'VPN', label: 'VPN' },
];

/** The mark beside a source's word; the word stays, so the icon is never the only sign (UR-7). */
export const SOURCE_ICONS: Readonly<Record<EventSource, IconName>> = {
  CHECK: 'carbon:task-complete',
  SESSION: 'carbon:terminal',
  DEVICE: 'carbon:bare-metal-server',
  ACCESS: 'carbon:security',
  HUB: 'carbon:network-3',
  VPN: 'carbon:vpn',
  METRICS: 'carbon:activity',
  NOTIFICATION: 'carbon:notification',
};

/** A source's mark; a source this client does not know yet gets a plain one. */
export function sourceIcon(source: string): IconName {
  return (SOURCE_ICONS as Partial<Record<string, IconName>>)[source] ?? 'carbon:catalog';
}

/** The text of a choice value; severity and source show as words, never as colour alone. */
export function labelOf<T extends string>(choices: readonly Choice<T>[], value: T): string {
  return choices.find((choice) => choice.value === value)?.label ?? value;
}

const pad = (part: number): string => String(part).padStart(2, '0');

/** A row's clock time in the operator's zone; the day it belongs to heads its group (`dayLabel`). */
export function clockLabel(at: string): string {
  const time = new Date(at);
  if (Number.isNaN(time.getTime())) return at;
  return `${pad(time.getHours())}:${pad(time.getMinutes())}:${pad(time.getSeconds())}`;
}

/** A time in the operator's zone with no day heading: the clock time today, the date and time before today. */
export function timeLabel(at: string, now: Date): string {
  const time = new Date(at);
  if (Number.isNaN(time.getTime())) return at;
  const clock = clockLabel(at);
  return time.toDateString() === now.toDateString() ? clock : `${pad(time.getMonth() + 1)}-${pad(time.getDate())} ${clock}`;
}

const DAY = new Intl.DateTimeFormat('en-GB', { weekday: 'short', day: 'numeric', month: 'short' });

/** The heading of a day's rows: Today, Yesterday, or the weekday and date; a time that does not parse has none. */
export function dayLabel(at: string, now: Date): string {
  const time = new Date(at);
  if (Number.isNaN(time.getTime())) return '';
  if (time.toDateString() === now.toDateString()) return 'Today';
  const yesterday = new Date(now);
  yesterday.setDate(now.getDate() - 1);
  return time.toDateString() === yesterday.toDateString() ? 'Yesterday' : DAY.format(time);
}

/** The device column: the name the hub sent, or a dash for an event of no device. */
export function deviceLabel(deviceName: string | null | undefined): string {
  return deviceName ?? '—';
}

/** `current` with the choice named `value` switched; a value that is not a choice leaves it as it is. */
export function switched<T extends string>(
  choices: readonly Choice<T>[],
  current: readonly T[],
  value: string,
): readonly T[] {
  const choice = choices.find((known) => known.value === value);
  return choice === undefined ? current : toggled(current, choice.value);
}

/** The filter with its time window set to `value`; a value that is not a window leaves it as it is (LOG-009). */
export function withSince(filter: LogFilter, value: string): LogFilter {
  const since = SINCE_CHOICES.find((known) => known.value === value)?.value;
  return since === undefined ? filter : { ...filter, since };
}

/** The filter narrowed to one device, or to all devices when `deviceId` is empty (LOG-009). */
export function withDevice(filter: LogFilter, deviceId: string): LogFilter {
  return { ...filter, deviceId: deviceId === '' ? null : deviceId };
}
