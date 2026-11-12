/**
 * The broker's audit run (vpn-plan.md Section 5.1, VPN-003, H2): read the nodes, check the ones
 * the broker is responsible for, apply the fixes, save the state. Every request starts with it,
 * and the audit timer (`anima-broker audit`) runs it alone, so a bad node is undone also while
 * the hub is silent. Headscale is reached only through `runner`, so tests pass a fake.
 */

import { tailnetOf, type BrokerTailnet } from './nodes.ts';
import { audit, fixStep, type Fix } from './outcome.ts';
import { Refused } from './protocol.ts';
import { readState, writeState, type BrokerFiles, type BrokerState } from './state.ts';
import { LIST_STEPS, type Step } from './verbs.ts';

/** Runs one Headscale step and returns its standard output; throws when the step fails. */
export type Runner = (step: Step) => Promise<string>;

/** A Headscale step failed, timed out or was not started. */
export class StepError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'StepError';
  }
}

/** What the log line reports: never a key, an auth ID or a node name (LOG-007). */
export interface Trace {
  node: number | null;
  fixes: number;
  unreadable: number;
}

export interface Run {
  readonly runner: Runner;
  readonly files: BrokerFiles;
  readonly now: Date;
  readonly trace: Trace;
}

export interface Settled {
  readonly state: BrokerState;
  readonly tailnet: BrokerTailnet;
  readonly fixes: readonly Fix[];
}

export const newTrace = (): Trace => ({ node: null, fixes: 0, unreadable: 0 });

/** The journald line for a run. */
export function logLine(verb: string, outcome: string, trace: Trace): string {
  const unreadable = trace.unreadable > 0 ? ` unreadable=${String(trace.unreadable)}` : '';
  return `anima-broker verb=${verb} outcome=${outcome} node=${trace.node === null ? '-' : String(trace.node)} fixes=${String(trace.fixes)}${unreadable}`;
}

export async function json(run: Run, step: Step): Promise<unknown> {
  return JSON.parse(await run.runner(step)) as unknown;
}

export async function readTailnet(run: Run): Promise<BrokerTailnet> {
  const [nodes, routes] = LIST_STEPS;
  if (nodes === undefined || routes === undefined) throw new Refused('headscale-failed');
  const tailnet = tailnetOf(await json(run, nodes), await json(run, routes));
  run.trace.unreadable = tailnet.unreadable.length;
  return tailnet;
}

/**
 * Audits the issued keys' nodes and the nodes of `state.pendingRegister`, applies the fixes,
 * then saves the state with `pendingRegister` cleared. A failed fix throws before the save, so
 * the next run audits the same nodes again.
 */
export async function settle(run: Run, state: BrokerState): Promise<Settled> {
  const before = await readTailnet(run);
  const result = audit(before, state.issued, state.pendingRegister, run.now);
  for (const fix of result.fixes) {
    await run.runner(fixStep(fix));
    run.trace.fixes += 1;
  }
  const next = { ...state, issued: result.issued, pendingRegister: null };
  await writeState(run.files.state, next);
  return { state: next, tailnet: result.fixes.length === 0 ? before : await readTailnet(run), fixes: result.fixes };
}

/** The audit timer's run: settle with the saved state; returns the log line. */
export async function auditTailnet(runner: Runner, files: BrokerFiles, now: Date): Promise<string> {
  const run: Run = { runner, files, now, trace: newTrace() };
  await settle(run, await readState(files.state));
  return logLine('audit', 'ok', run.trace);
}
