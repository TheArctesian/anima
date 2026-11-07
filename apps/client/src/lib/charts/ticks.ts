import { spanOf, timeScale, valueY, type TimeWindow, type ValueScale } from './scales';

/** A label on the time axis at `x` (viewBox units). */
export interface TimeTick {
  readonly at: number;
  readonly x: number;
  readonly text: string;
}

/** A horizontal grid line at `y` (viewBox units) and the value it marks. */
export interface ValueTick {
  readonly value: number;
  readonly y: number;
}

const MINUTE = 60_000;
const HOUR = 60 * MINUTE;
const DAY = 24 * HOUR;

/** Candidate distances between time ticks; the smallest that gives at most `MAX_TIME_TICKS` intervals wins. */
const TIME_STEPS = [MINUTE, 5 * MINUTE, 10 * MINUTE, 15 * MINUTE, 30 * MINUTE, HOUR, 2 * HOUR, 3 * HOUR, 4 * HOUR, 6 * HOUR, 12 * HOUR, DAY, 2 * DAY, 7 * DAY];
const MAX_TIME_TICKS = 7;
const WEEKDAYS = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'] as const;

const twoDigits = (value: number) => String(value).padStart(2, '0');

/** The tick text for local time `local` (ms, already shifted to the zone): "HH:MM" within a day, "Thu 5" for days. */
function tickText(local: number, step: number): string {
  const date = new Date(local);
  if (step < DAY) return `${twoDigits(date.getUTCHours())}:${twoDigits(date.getUTCMinutes())}`;
  return `${WEEKDAYS[date.getUTCDay()] ?? ''} ${String(date.getUTCDate())}`;
}

/**
 * Time-axis labels inside `window` (ms), on round local times: every 10 minutes over 1 hour, every 4 hours over
 * 24 hours, every day at midnight over 7 days. `zoneOffset` is the local zone's distance from UTC in ms (east is
 * positive). The edges themselves get no label. A window wider than the largest step gets none.
 */
export function timeTicks(window: TimeWindow, zoneOffset: number): readonly TimeTick[] {
  const span = spanOf(window);
  const step = TIME_STEPS.find((candidate) => span / candidate <= MAX_TIME_TICKS);
  if (span === 0 || step === undefined || !Number.isFinite(zoneOffset)) return [];
  const x = timeScale(window);
  const ticks: TimeTick[] = [];
  for (let local = Math.floor((window.start + zoneOffset) / step + 1) * step; local - zoneOffset < window.end; local += step) {
    const at = local - zoneOffset;
    ticks.push({ at, x: x(at), text: tickText(local, step) });
  }
  return ticks;
}

/** The value of each horizontal grid line, 0 at the bottom first, up to the scale's top. */
export function valueTicks(scale: ValueScale): readonly ValueTick[] {
  const count = Math.round(scale.top / scale.step);
  return Array.from({ length: count + 1 }, (_, index) => {
    const value = Number((index * scale.step).toPrecision(12));
    return { value, y: valueY(scale, value) };
  });
}
