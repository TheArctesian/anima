import { toggled } from '$lib/devices/toolbar/toolbar';
import { SINCE_CHOICES, type Choice } from './view';
import type { LogFilter } from '../log';

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
