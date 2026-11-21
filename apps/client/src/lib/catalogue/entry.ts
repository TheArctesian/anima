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
