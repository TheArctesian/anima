import type { EventSource, Severity } from '$lib/hub/generated/graphql';
import type { Since } from '../log';

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

/** The text of a choice value; severity and source show as words, never as colour alone. */
export function labelOf<T extends string>(choices: readonly Choice<T>[], value: T): string {
  return choices.find((choice) => choice.value === value)?.label ?? value;
}

const pad = (part: number): string => String(part).padStart(2, '0');

/** A row's time in the operator's zone: the clock time today, the date and time before today. */
export function timeLabel(at: string, now: Date): string {
  const time = new Date(at);
  if (Number.isNaN(time.getTime())) return at;
  const clock = `${pad(time.getHours())}:${pad(time.getMinutes())}:${pad(time.getSeconds())}`;
  return time.toDateString() === now.toDateString() ? clock : `${pad(time.getMonth() + 1)}-${pad(time.getDate())} ${clock}`;
}

/** The device column: the name the hub sent, or a dash for an event of no device. */
export function deviceLabel(deviceName: string | null | undefined): string {
  return deviceName ?? '—';
}
