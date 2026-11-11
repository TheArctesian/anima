/**
 * The broker's own limits (vpn-plan.md Section 5.1): at most `perDay` pre-auth keys and
 * `perDay` registrations in 24 hours, and no new node at the node limit. They hold also when
 * the hub is lost, because the hub cannot change them. Pure; time is a parameter.
 */

import type { Verb } from './protocol.ts';

/** ISO times of the keys and registrations made in the last 24 hours. */
export interface LimitState {
  readonly keys: readonly string[];
  readonly registrations: readonly string[];
}

export interface LimitConfig {
  readonly perDay: number;
  readonly maxNodes: number;
}

export interface Usage {
  readonly state: LimitState;
  readonly nodeCount: number;
}

const DAY_MS = 86_400_000;

type Counted = keyof LimitState;

function counted(verb: Verb): Counted | null {
  if (verb === 'personal-key' || verb === 'router-key') return 'keys';
  if (verb === 'register') return 'registrations';
  return null;
}

function recent(times: readonly string[], now: Date): readonly string[] {
  return times.filter((time) => {
    const age = now.getTime() - Date.parse(time);
    // An unreadable or future time still counts: the broker never forgets a key by accident.
    return Number.isNaN(age) || age <= DAY_MS;
  });
}

/** Whether the verb may run now. `nodes`, `approve-route` and `renew` are never limited. */
export function admit(verb: Verb, usage: Usage, config: LimitConfig, now: Date): 'ok' | 'rate-limited' | 'node-limit' {
  const kind = counted(verb);
  if (kind === null) return 'ok';
  if (usage.nodeCount >= config.maxNodes) return 'node-limit';
  if (recent(usage.state[kind], now).length >= config.perDay) return 'rate-limited';
  return 'ok';
}

/** The state after the verb ran at `now`; drops entries older than 24 hours. */
export function record(verb: Verb, state: LimitState, now: Date): LimitState {
  const kind = counted(verb);
  const kept = { keys: recent(state.keys, now), registrations: recent(state.registrations, now) };
  if (kind === null) return kept;
  return { ...kept, [kind]: [...kept[kind], now.toISOString()] };
}
