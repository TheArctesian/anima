import type { NewHeartbeat } from './api';

/** How many cells the bar has (Section 6.1). */
export const BAR_LENGTH = 50;

/** One heartbeat on a device's bar, with the check that recorded it. */
export interface Beat {
  readonly checkId: string;
  readonly at: string;
  readonly status: NewHeartbeat['heartbeat']['status'];
  readonly responseTimeMs?: number | null;
}

const NO_UPTIME = '—';

/** Time order; a time that does not parse sorts as the oldest. */
function when(beat: Beat): number {
  const time = Date.parse(beat.at);
  return Number.isNaN(time) ? Number.NEGATIVE_INFINITY : time;
}

/**
 * Heartbeats have no id, so a beat is identified by its check and its instant: one check
 * records at most one heartbeat per instant. Instants compare as times, not as text, so
 * `Z` and `+00:00` spellings of one moment are the same beat.
 * A time that does not parse counts as the oldest instant, so two unparseable beats of one
 * check collapse into one: nothing tells them apart, and the hub never sends such a time.
 */
function sameBeat(a: Beat, b: Beat): boolean {
  return a.checkId === b.checkId && when(a) === when(b);
}

/** The last `BAR_LENGTH` of beats already in time order. */
function capped(beats: readonly Beat[]): readonly Beat[] {
  return beats.length > BAR_LENGTH ? beats.slice(-BAR_LENGTH) : beats;
}

/** A check with its recent heartbeats: the shape `latestBeats` reads from any document that asks for them. */
interface CheckBeats {
  readonly id: string;
  readonly heartbeats: { readonly edges: readonly { readonly node: Omit<Beat, 'checkId'> }[] };
}

/** The device's last `BAR_LENGTH` heartbeats, oldest first, merged from all its checks (D-6). Server order is not trusted. */
export function latestBeats(checks: readonly CheckBeats[]): readonly Beat[] {
  const all = checks.flatMap((check) =>
    check.heartbeats.edges.map(({ node }): Beat => ({ checkId: check.id, ...node })),
  );
  const unique = all.filter((beat, index) => all.findIndex((other) => sameBeat(other, beat)) === index);
  return capped(unique.toSorted((a, b) => when(a) - when(b)));
}

/**
 * Adds one live beat in time order and keeps the last `BAR_LENGTH`. A duplicate, or a beat older
 * than a full bar, returns `beats` itself so a caller can see that nothing changed.
 */
export function addBeat(beats: readonly Beat[], beat: Beat): readonly Beat[] {
  if (beats.some((known) => sameBeat(known, beat))) return beats;
  const time = when(beat);
  const after = beats.findLastIndex((known) => when(known) <= time) + 1;
  if (after === 0 && beats.length >= BAR_LENGTH) return beats;
  return capped([...beats.slice(0, after), beat, ...beats.slice(after)]);
}

/** A pushed heartbeat as a bar beat. */
export function beatOf(added: NewHeartbeat): Beat {
  return { checkId: added.checkId, ...added.heartbeat };
}

/** The bar's cells, oldest first: a status per beat, and `null` for each empty cell on the left. */
export function cellsOf(beats: readonly Beat[]): readonly (Beat['status'] | null)[] {
  const empty = Array.from({ length: Math.max(BAR_LENGTH - beats.length, 0) }, () => null);
  return [...empty, ...beats.slice(-BAR_LENGTH).map((beat) => beat.status)];
}

/** What a screen reader hears for the bar: how many beats and the latest status. */
export function barSummary(beats: readonly Beat[]): string {
  const latest = beats.at(-1);
  if (latest === undefined) return 'No heartbeats yet';
  const count = beats.length === 1 ? '1 heartbeat' : `${String(beats.length)} heartbeats`;
  return `${count}, latest ${latest.status.toLowerCase()}`;
}

/**
 * Whether `now` has a newer last beat than `was`: the bar then moves one cell left.
 * A bar that showed no beat is being loaded, not fed, so it does not move.
 */
export function shiftedByNewBeat(was: readonly Beat[], now: readonly Beat[]): boolean {
  const latest = now.at(-1);
  const previous = was.at(-1);
  return latest !== undefined && previous !== undefined && !sameBeat(previous, latest);
}

/**
 * An uptime ratio (0 to 1) as a percentage with two decimals, "99.95 %"; "—" when there is none.
 * Rounding never shows "100.00 %" for an uptime below 1: it shows "99.99 %".
 */
export function uptimeText(ratio: number | null): string {
  if (ratio === null || Number.isNaN(ratio)) return NO_UPTIME;
  const percent = Math.min(Math.max(ratio, 0), 1) * 100;
  const text = percent.toFixed(2);
  return text === '100.00' && percent < 100 ? '99.99 %' : `${text} %`;
}
