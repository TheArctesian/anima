import type { Cast } from './parse';

/** The recording's length in seconds: the time of its last event. */
export function durationOf(cast: Cast): number {
  return cast.events.at(-1)?.at ?? 0;
}

/** How many events have happened by `time` (those at exactly `time` included). */
export function eventsBy(cast: Cast, time: number): number {
  let low = 0;
  let high = cast.events.length;
  while (low < high) {
    const middle = (low + high) >> 1;
    if ((cast.events[middle]?.at ?? Infinity) <= time) low = middle + 1;
    else high = middle;
  }
  return low;
}

/** `m:ss`, or `h:mm:ss` from one hour; a time below zero or not a number reads `0:00`. */
export function clockText(seconds: number): string {
  const whole = Number.isFinite(seconds) && seconds > 0 ? Math.floor(seconds) : 0;
  const pad = (n: number) => String(n).padStart(2, '0');
  const hours = Math.floor(whole / 3600);
  const minutes = Math.floor((whole % 3600) / 60);
  const rest = pad(whole % 60);
  return hours > 0 ? `${String(hours)}:${pad(minutes)}:${rest}` : `${String(minutes)}:${rest}`;
}
