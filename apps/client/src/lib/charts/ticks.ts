import { lengthOf, timeScale, valueY, type TimeWindow, type ValueScale } from './scales';

/** A label on the time axis at `x` (viewBox units). */
interface TimeTick {
  readonly at: number;
  readonly x: number;
  readonly text: string;
}

/** A horizontal grid line at `y` (viewBox units) and the value it marks. */
interface ValueTick {
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

/** The UTC time of local time `local`, for a zone whose offset can change (daylight saving). */
function utcOf(local: number, offsetAt: (at: number) => number): number {
  const guess = local - offsetAt(local);
  return local - offsetAt(guess);
}

/**
 * Time-axis labels inside `span` (ms), on round local times: every 10 minutes over 1 hour, every 4 hours over
 * 24 hours, every day at midnight over 7 days. `offsetAt(at)` is the local zone's distance from UTC in ms at `at`
 * (east is positive), so a span across a daylight-saving change keeps its labels on local times. The edges get no
 * label; each instant gets one label. A span longer than the largest step, or a zone offset that
 * is not a number, gets none.
 */
export function timeTicks(span: TimeWindow, offsetAt: (at: number) => number): readonly TimeTick[] {
  const length = lengthOf(span);
  const step = TIME_STEPS.find((candidate) => length / candidate <= MAX_TIME_TICKS);
  if (length === 0 || step === undefined || !Number.isFinite(offsetAt(span.start))) return [];
  const x = timeScale(span);
  const ticks: TimeTick[] = [];
  let local = Math.floor((span.start + offsetAt(span.start)) / step + 1) * step;
  // A change of offset moves a tick by at most a few hours, so this bound is never reached by a real zone.
  for (let count = 0; count <= 2 * MAX_TIME_TICKS; count += 1, local += step) {
    const at = utcOf(local, offsetAt);
    if (!Number.isFinite(at) || at >= span.end) break;
    // A local time skipped when the clocks go forward lands on the next one; the later, true label wins.
    if (ticks.at(-1)?.at === at) ticks.pop();
    if (at > span.start) ticks.push({ at, x: x(at), text: tickText(local, step) });
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
