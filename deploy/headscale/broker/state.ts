/**
 * The broker's files (R-055, VP-8): its config, the policy it reads, and the state that
 * survives between requests. `MaxConnections=1` runs one request at a time, so no lock is
 * needed; the state is written to a temporary file and renamed, so a kill never leaves half.
 */

import { open, readFile, rename, rm } from 'node:fs/promises';
import { parseHujson } from '../src/hujson.ts';
import { parsePolicy } from '../src/policy.ts';
import type { LimitState } from './limits.ts';
import type { Issued } from './outcome.ts';
import { policyView, type PolicyView } from './verbs.ts';

export interface BrokerFiles {
  /** `/etc/anima/broker.conf`: `max_nodes=<n>` and nothing else. */
  readonly config: string;
  readonly policy: string;
  /** `/var/lib/anima-broker/state.json`. */
  readonly state: string;
}

export interface BrokerState extends LimitState {
  readonly issued: readonly Issued[];
  /** The node a killed `register` request left unchecked. */
  readonly pendingRegister: number | null;
}

export class StateError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'StateError';
  }
}

export const EMPTY_STATE: BrokerState = { keys: [], registrations: [], issued: [], pendingRegister: null };

const MAX_NODES = /^max_nodes=([1-9]\d{0,4})$/;

/** Reads `max_nodes` from the config text; any other line is an error. */
export function maxNodesOf(text: string): number {
  const lines = text.split('\n').map((line) => line.trim()).filter((line) => line !== '' && !line.startsWith('#'));
  const match = lines.length === 1 ? MAX_NODES.exec(lines[0] ?? '') : null;
  if (match?.[1] === undefined) throw new StateError('broker.conf must hold one line max_nodes=<n>');
  return Number(match[1]);
}

export async function readPolicyView(path: string): Promise<PolicyView> {
  return policyView(parsePolicy(parseHujson(await readFile(path, 'utf8'))));
}

const isTime = (value: unknown): value is string => typeof value === 'string';
const isId = (value: unknown): value is number => typeof value === 'number' && Number.isSafeInteger(value) && value > 0;

function isIssued(value: unknown): value is Issued {
  if (typeof value !== 'object' || value === null) return false;
  const entry = value as Record<string, unknown>;
  const site = entry.site === undefined || typeof entry.site === 'string';
  return isId(entry.keyId) && (entry.verb === 'personal-key' || entry.verb === 'router-key') && site && isTime(entry.at) && typeof entry.passed === 'boolean';
}

/** Parses the state file text; a state the broker did not write is an error, never "empty". */
export function parseState(text: string): BrokerState {
  let value: unknown;
  try {
    value = JSON.parse(text);
  } catch {
    throw new StateError('state.json is not JSON');
  }
  const state = (typeof value === 'object' && value !== null ? value : {}) as Record<string, unknown>;
  const { keys, registrations, issued, pendingRegister } = state;
  const lists = [keys, registrations].every((list) => Array.isArray(list) && list.every(isTime));
  const entries = Array.isArray(issued) && issued.every(isIssued);
  if (!lists || !entries || !(pendingRegister === null || isId(pendingRegister))) throw new StateError('state.json has an unknown shape');
  return { keys, registrations, issued, pendingRegister } as BrokerState;
}

/** The saved state, or the empty state when the broker has never run. */
export async function readState(path: string): Promise<BrokerState> {
  let text: string;
  try {
    text = await readFile(path, 'utf8');
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code === 'ENOENT') return EMPTY_STATE;
    throw error;
  }
  return parseState(text);
}

/** Writes the state to a temporary file, flushes it, then renames it over the old one. */
export async function writeState(path: string, state: BrokerState): Promise<void> {
  const temporary = `${path}.${String(process.pid)}.tmp`;
  const file = await open(temporary, 'w', 0o600);
  try {
    await file.writeFile(`${JSON.stringify(state)}\n`);
    await file.sync();
  } finally {
    await file.close();
  }
  try {
    await rename(temporary, path);
  } catch (error) {
    await rm(temporary, { force: true });
    throw error;
  }
}
