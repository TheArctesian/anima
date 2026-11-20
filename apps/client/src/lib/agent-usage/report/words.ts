import type { AgentSource, AgentUsageState } from '$lib/hub/generated/graphql';
import type { UsageAnswer, UsageDevice, UsageSums } from '../api';
import { NO_PRICE, dollars, tokens } from './units';

// The words of the page. Where the command says the same thing, the wording is the command's (report.rs, AIS-021, AIS-024).

/** The sentence for a device whose daemon runs as the systemd system unit (AIS-024). */
export const SYSTEM_UNIT = 'This daemon runs as a system unit and cannot see the sessions of a user.';

/** The sentence for a device where the counts are off in the daemon's settings. */
export const TURNED_OFF = 'The use counts are turned off in the settings of the daemon on this device.';

const SOURCE_NAMES: Readonly<Record<AgentSource, string>> = { CLAUDE_CODE: 'Claude Code', CODEX: 'Codex' };

/** The name of a program; one this client does not know yet shows as the hub names it. */
export function sourceName(source: AgentSource): string {
  return (SOURCE_NAMES as Partial<Record<string, string>>)[source] ?? source;
}

/** The sentence for a daemon that sends no counts; `null` for one that reads. */
export function stateSentence(state: AgentUsageState): string | null {
  if (state === 'SYSTEM_UNIT') return SYSTEM_UNIT;
  return state === 'TURNED_OFF' ? TURNED_OFF : null;
}

/** All the tokens of a group: input, output and both cache counts. */
export function totalTokens(sums: UsageSums): number {
  return sums.inputTokens + sums.outputTokens + sums.cacheReadTokens + sums.cacheWriteTokens;
}

/** The estimate of a group in US dollars, or the words that say it has no price. */
export function costText(sums: UsageSums): string {
  return sums.estimatedCostUsd == null ? NO_PRICE : dollars(sums.estimatedCostUsd);
}

/** The one line over the table: sessions, tokens and the estimate, as text. */
export function summaryLine(total: UsageSums): string {
  const sessions = `${String(total.sessions)} ${total.sessions === 1 ? 'session' : 'sessions'}`;
  const cost = total.estimatedCostUsd == null ? 'no estimated cost' : `estimated cost $${dollars(total.estimatedCostUsd)}`;
  return `${sessions}, ${tokens(totalTokens(total))} tokens, ${cost}`;
}

/** What the estimate is, and what it is not (AIS-021). */
export function estimateNote(pricesAsOf: string): string {
  return `The cost is an estimate from the public list prices of ${pricesAsOf}. It is not the bill of a subscription.`;
}

/** The lines under the table, as text: the estimate, the tokens it leaves out, and the rows the hub cut. */
export function notes(answer: UsageAnswer): string[] {
  const lines = [estimateNote(answer.pricesAsOf)];
  if (answer.total.unpricedTokens > 0) {
    lines.push(`${tokens(answer.total.unpricedTokens)} tokens have no price and are not in the estimate.`);
  }
  if (answer.rowsLeftOut > 0) {
    lines.push(answer.rowsLeftOut === 1 ? '1 row is not shown.' : `${String(answer.rowsLeftOut)} rows are not shown.`);
  }
  return lines;
}

/** One entry for each device whose daemon sends no counts, with the reason; a device that reads gives none. */
export function deviceNotes(devices: readonly UsageDevice[]): { readonly id: string; readonly name: string; readonly sentence: string }[] {
  return devices.flatMap(({ device, state }) => {
    const sentence = stateSentence(state);
    return sentence === null ? [] : [{ id: device.id, name: device.name, sentence }];
  });
}
