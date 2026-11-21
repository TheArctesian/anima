import type { CatalogueEntry } from './api';

/** The words of a K1 enum value: `NETWORK_PORT` as "Network port". */
export function enumWords(value: string): string {
  const words = value.toLowerCase().replaceAll('_', ' ');
  return words.charAt(0).toUpperCase() + words.slice(1);
}

/** HOST-047: the day of the last test on a machine, or the words that say there was none. */
export function machineTest(entry: Pick<CatalogueEntry, 'lastMachineTest'>): string {
  return entry.lastMachineTest === null ? 'Not tested on a machine' : `Passed on a machine on ${entry.lastMachineTest}`;
}

/** The marks of an entry in words, with the ports on the network; "None" for an entry with no mark. */
export function markWords(entry: Pick<CatalogueEntry, 'marks' | 'networkPorts'>): string {
  if (entry.marks.length === 0) return 'None';
  const ports = entry.networkPorts.map((open) => `${String(open.port)} (${open.protocols.join(', ')})`);
  return entry.marks.map((mark) => (mark === 'NETWORK_PORT' && ports.length > 0 ? `Network port ${ports.join(', ')}` : enumWords(mark))).join('; ');
}

/** The tags that name a publisher, not what the entry is (task H39): they never give the category. */
const PUBLISHER_TAGS: readonly string[] = ['linuxserver'];

/** OD-HOST-54: why a shell needs more than an application, in the words of the list. */
export const SHELL_REASON = 'Needs a password that the machine makes. It has no public route.';

/** The choice of the tier above the list: each entry, or one tier (OD-HOST-50). */
export type TierChoice = 'ALL' | CatalogueEntry['tier'];

/** The label of each choice of the tier. */
export const TIER_LABELS: Readonly<Record<TierChoice, string>> = { ALL: 'All', TESTED: 'Tested by Anima', GENERATED: 'Generated' };

/** The group of an entry in the list: its first tag that does not name a publisher, as words; "Other" when none. */
export function category(entry: Pick<CatalogueEntry, 'tags'>): string {
  const tag = entry.tags.find((each) => !PUBLISHER_TAGS.includes(each.toLowerCase()));
  return tag === undefined ? 'Other' : tag.charAt(0).toUpperCase() + tag.slice(1);
}

/** True when the name or a tag of the entry holds the search, in any case; an empty search holds each entry. */
export function matches(entry: Pick<CatalogueEntry, 'name' | 'tags'>, search: string): boolean {
  const wanted = search.trim().toLowerCase();
  if (wanted === '') return true;
  return entry.name.toLowerCase().includes(wanted) || entry.tags.some((tag) => tag.toLowerCase().includes(wanted));
}

/** The entries of one category, in the order of their names. */
export interface Group<Entry> {
  category: string;
  entries: Entry[];
}

/** The entries that the search and the tier keep, in groups by category, each in alphabetical order. */
export function grouped<Entry extends Pick<CatalogueEntry, 'name' | 'tags' | 'tier'>>(entries: readonly Entry[], search: string, tier: TierChoice): Group<Entry>[] {
  const groups = new Map<string, Entry[]>();
  for (const entry of entries) {
    if ((tier !== 'ALL' && entry.tier !== tier) || !matches(entry, search)) continue;
    const name = category(entry);
    groups.set(name, [...(groups.get(name) ?? []), entry]);
  }
  const byName = (left: string, right: string) => left.localeCompare(right, 'en', { sensitivity: 'base' });
  return [...groups.entries()]
    .sort(([left], [right]) => byName(left, right))
    .map(([name, members]) => ({ category: name, entries: members.sort((left, right) => byName(left.name, right.name)) }));
}

/** How many entries each choice of the tier holds. */
export function tierCounts(entries: readonly Pick<CatalogueEntry, 'tier'>[]): Record<TierChoice, number> {
  const tested = entries.filter((entry) => entry.tier === 'TESTED').length;
  return { ALL: entries.length, TESTED: tested, GENERATED: entries.length - tested };
}

function counted(count: number, one: string, many: string): string {
  return `${String(count)} ${count === 1 ? one : many}`;
}

/** The second line of a row: the version, the kind when it is not an application, and the streams and optional parts. */
export function rowLine(entry: Pick<CatalogueEntry, 'version' | 'kind' | 'streams' | 'optionalParts'>): string {
  const parts = [entry.version];
  if (entry.kind !== 'APPLICATION') parts.push(enumWords(entry.kind));
  if (entry.streams.length > 0) parts.push(counted(entry.streams.length, 'stream', 'streams'));
  if (entry.optionalParts.length > 0) parts.push(counted(entry.optionalParts.length, 'optional part', 'optional parts'));
  return parts.join(' · ');
}

/** The tier as a fact: where a generated entry comes from, or that the Anima project wrote it. */
export function tierFact(entry: Pick<CatalogueEntry, 'origin' | 'lastMachineTest'>): string {
  if (entry.origin !== null) return `From ${entry.origin.publisher}, data of ${entry.origin.read}. Anima did not test it.`;
  return `Written and read by the Anima project. ${machineTest(entry)}.`;
}

/** What "Not reviewed" means for the person, in one sentence that the description shows. */
export const UNREVIEWED_NOTE = 'No person has read the settings of this entry. Read them before you install it.';

/** True for a generated entry that no person reviewed: the list and the description say so. */
export function unreviewed(entry: Pick<CatalogueEntry, 'origin' | 'reviewedBy'>): boolean {
  return entry.origin !== null && entry.reviewedBy === null;
}

/** Who read a generated entry and when, as a fact; null for an entry that the Anima project wrote. */
export function reviewFact(entry: Pick<CatalogueEntry, 'origin' | 'reviewedBy' | 'reviewedOn'>): string | null {
  if (entry.origin === null) return null;
  if (entry.reviewedBy === null) return 'Not reviewed';
  return `Reviewed by ${entry.reviewedBy} on ${entry.reviewedOn ?? 'an unknown day'}`;
}

/** The protocols of a stream in words: `['tcp', 'udp']` as "TCP and UDP". */
export function protocolWords(protocols: readonly string[]): string {
  return protocols.map((protocol) => protocol.toUpperCase()).join(' and ');
}

/** The state of an optional part: off by default, or on. */
export function partState(part: Pick<CatalogueEntry['optionalParts'][number], 'on'>): string {
  return part.on ? 'On' : 'Off by default';
}
