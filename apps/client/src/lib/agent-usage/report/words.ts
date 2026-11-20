import type { AgentSource, AgentUsageState } from '$lib/hub/generated/graphql';
import type { UsageAnswer, UsageDevice, UsageSums } from '../api';
import { NO_PRICE, dollars, tokens } from './units';

// The words of the page. Where the command says the same thing, the wording is the command's (report.rs, AIS-021, AIS-024).

const SOURCE_NAMES: Readonly<Record<AgentSource, string>> = { CLAUDE_CODE: 'Claude Code', CODEX: 'Codex' };

/** The name of a program; one this client does not know yet shows as the hub names it. */
export function sourceName(source: AgentSource | null): string {
  if (source === null) return 'No program';
  return (SOURCE_NAMES as Partial<Record<string, string>>)[source] ?? source;
}

/** What the daemon of a device does with the sessions (AIS-024), and whether that is the working state. */
export function stateWords(state: AgentUsageState): { readonly words: string; readonly reads: boolean } {
  if (state === 'SYSTEM_UNIT') return { words: "Runs as a system unit: it cannot see a user's sessions", reads: false };
  if (state === 'TURNED_OFF') return { words: "Turned off in the daemon's settings", reads: false };
  return { words: 'Reads sessions', reads: true };
}

/** The estimate of a group in US dollars, or the words that say it has no price. */
export function costText(sums: Pick<UsageSums, 'estimatedCostUsd'>): string {
  return sums.estimatedCostUsd == null ? NO_PRICE : `$${dollars(sums.estimatedCostUsd)}`;
}

/** What the estimate is, and what it is not (AIS-021), with the tokens that it leaves out when there are some. */
export function estimateNote(pricesAsOf: string, unpricedTokens: number): string {
  const base = `list prices of ${pricesAsOf}; not a bill`;
  return unpricedTokens > 0 ? `${base}. ${tokens(unpricedTokens)} tokens have no known price` : base;
}

/** The line under the list of repositories when the hub left some out. */
export function leftOutLine(count: number): string {
  if (count <= 0) return '';
  return `${String(count)} more ${count === 1 ? 'group is' : 'groups are'} not shown. The totals count them.`;
}

// The two functions below serve only `UsageNotes.svelte`, which the page no longer draws. That file and
// `report/columns.ts` are not used any more and wait for a person to take them out of the repository.

/** The lines under the old table. */
export function notes(answer: UsageAnswer): string[] {
  return [estimateNote(answer.pricesAsOf, answer.total.unpricedTokens), leftOutLine(answer.rowsLeftOut)].filter((line) => line !== '');
}

/** One entry for each device whose daemon sends no counts, with the reason. */
export function deviceNotes(devices: readonly UsageDevice[]): { readonly id: string; readonly name: string; readonly sentence: string }[] {
  return devices.flatMap(({ device, state }) => {
    const { words, reads } = stateWords(state);
    return reads ? [] : [{ id: device.id, name: device.name, sentence: words }];
  });
}
