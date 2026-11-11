/**
 * One broker request, from the request line to the reply (R-055, VP-7, VP-8): check the
 * nodes the broker is responsible for, check the limits, run the verb, check its result.
 * Headscale is reached only through `runner`, so tests pass a fake. The pre-auth key goes
 * only into the reply; the log line holds the verb, the outcome, a node id and a count.
 */

import { readFile } from 'node:fs/promises';
import type { Tailnet } from '../src/inventory.ts';
import { admit, record } from './limits.ts';
import { newKeyOf, nodesReply, operatorIdOf, registered, registeredIdOf, tailnetOf } from './nodes.ts';
import { audit, fixStep, type Fix, type Issued } from './outcome.ts';
import { parseRequest, type Refusal, type Reply, type Request, type Verb } from './protocol.ts';
import { maxNodesOf, readPolicyView, readState, writeState, type BrokerFiles, type BrokerState } from './state.ts';
import { LIST_STEPS, plan, USERS_STEP, type PolicyView, type Step } from './verbs.ts';

/** Runs one Headscale step and returns its standard output; throws when the step fails. */
export type Runner = (step: Step) => Promise<string>;

export interface Served {
  readonly reply: Reply<unknown>;
  /** The journald line: never a key, an auth ID or a node name (LOG-007). */
  readonly log: string;
}

const PER_DAY = 3;

const DETAILS: Readonly<Record<Refusal, string>> = {
  'bad-request': 'the request is not one of the broker verbs',
  'unknown-site': 'the site or its tag is not in the VPN policy',
  'rate-limited': 'at most 3 keys and 3 registrations in 24 hours',
  'node-limit': 'the tailnet is at max_nodes (/etc/anima/broker.conf)',
  'not-allowed': 'the node does not qualify for this verb',
  undone: 'the new node was deleted',
  'headscale-failed': 'Headscale or a broker file failed',
};

class Refused extends Error {
  readonly refusal: Refusal;

  constructor(refusal: Refusal, detail = DETAILS[refusal]) {
    super(detail);
    this.refusal = refusal;
  }
}

interface Trace {
  node: number | null;
  fixes: number;
}

interface Run {
  readonly runner: Runner;
  readonly files: BrokerFiles;
  readonly now: Date;
  readonly trace: Trace;
}

interface Settled {
  readonly state: BrokerState;
  readonly tailnet: Tailnet;
  readonly fixes: readonly Fix[];
}

async function json(run: Run, step: Step): Promise<unknown> {
  return JSON.parse(await run.runner(step)) as unknown;
}

async function readTailnet(run: Run): Promise<Tailnet> {
  const [nodes, routes] = LIST_STEPS;
  if (nodes === undefined || routes === undefined) throw new Refused('headscale-failed');
  return tailnetOf(await json(run, nodes), await json(run, routes));
}

/** Audits the broker's nodes, applies the fixes, saves the state; `registered` is checked too. */
async function settle(run: Run, state: BrokerState, registeredId: number | null): Promise<Settled> {
  const before = await readTailnet(run);
  const result = audit(before, state.issued, registeredId, run.now);
  for (const fix of result.fixes) {
    await run.runner(fixStep(fix));
    run.trace.fixes += 1;
  }
  const next = { ...state, issued: result.issued, pendingRegister: null };
  await writeState(run.files.state, next);
  return { state: next, tailnet: result.fixes.length === 0 ? before : await readTailnet(run), fixes: result.fixes };
}

async function single(run: Run, steps: readonly Step[]): Promise<string> {
  const [step] = steps;
  if (step === undefined || steps.length !== 1) throw new Refused('headscale-failed');
  return run.runner(step);
}

async function newKey(run: Run, request: Request & { verb: 'personal-key' | 'router-key' }, state: BrokerState, steps: readonly Step[]): Promise<unknown> {
  const key = newKeyOf(JSON.parse(await single(run, steps)));
  const entry: Issued = { keyId: key.id, verb: request.verb, ...(request.verb === 'router-key' ? { site: request.site } : {}), at: run.now.toISOString(), passed: false };
  await writeState(run.files.state, { ...state, issued: [...state.issued, entry] });
  return { key: key.key, expiresAt: key.expiresAt.toISOString(), keyId: key.id };
}

async function register(run: Run, state: BrokerState, steps: readonly Step[]): Promise<unknown> {
  // The node id comes from the reply, never from a difference of two lists (another node may join).
  const id = registeredIdOf(JSON.parse(await single(run, steps)));
  run.trace.node = id;
  const pending = { ...state, pendingRegister: id };
  await writeState(run.files.state, pending);
  const settled = await settle(run, pending, id);
  const undone = settled.fixes.find((fix) => fix.node === id && fix.kind === 'delete');
  if (undone?.kind === 'delete') throw new Refused('undone', `${DETAILS.undone}: ${undone.reason}`);
  return answerFor(settled.tailnet, id);
}

function answerFor(tailnet: Tailnet, id: number): unknown {
  const node = tailnet.nodes.find((candidate) => candidate.id === id);
  if (node === undefined) throw new Refused('headscale-failed', 'the node is not in the node list');
  return registered(node);
}

async function perform(run: Run, request: Request, state: BrokerState, steps: readonly Step[]): Promise<unknown> {
  switch (request.verb) {
    case 'nodes':
      // Answered in `answer` from the list it already read.
      throw new Refused('headscale-failed');
    case 'personal-key':
    case 'router-key':
      return newKey(run, request, state, steps);
    case 'approve-route':
      await single(run, steps);
      return { node: request.node, site: request.site };
    case 'register':
      return register(run, state, steps);
    case 'renew':
      await single(run, steps);
      return answerFor(await readTailnet(run), request.node);
  }
}

async function answer(request: Request, run: Run, policy: PolicyView): Promise<unknown> {
  const maxNodes = maxNodesOf(await readFile(run.files.config, 'utf8'));
  // A request killed after `auth register` left its node unchecked: check it first.
  const saved = await readState(run.files.state);
  const settled = await settle(run, saved, saved.pendingRegister);
  if (request.verb === 'nodes') return nodesReply(settled.tailnet, policy);
  const operatorId = request.verb === 'personal-key' ? operatorIdOf(await json(run, USERS_STEP)) : null;
  const steps = plan(request, policy, nodesReply(settled.tailnet, policy).nodes, { now: run.now, operatorId });
  if (typeof steps === 'string') throw new Refused(steps);
  const verdict = admit(request.verb, { state: settled.state, nodeCount: settled.tailnet.nodes.length }, { perDay: PER_DAY, maxNodes }, run.now);
  if (verdict !== 'ok') throw new Refused(verdict);
  // The count is written before the step, so a kill after the step cannot leave a key uncounted.
  const counted = { ...settled.state, ...record(request.verb, settled.state, run.now) };
  await writeState(run.files.state, counted);
  return perform(run, request, counted, steps);
}

function served(verb: Verb | '-', reply: Reply<unknown>, trace: Trace): Served {
  const outcome = reply.ok ? 'ok' : reply.refusal;
  return { reply, log: `anima-broker verb=${verb} outcome=${outcome} node=${trace.node === null ? '-' : String(trace.node)} fixes=${String(trace.fixes)}` };
}

const refusal = (refused: Refusal, detail = DETAILS[refused]): Reply<unknown> => ({ ok: false, refusal: refused, detail });

/** Answers one request line. Every failure is a refusal; nothing here throws. */
export async function serve(line: string, runner: Runner, files: BrokerFiles, now: Date): Promise<Served> {
  const trace: Trace = { node: null, fixes: 0 };
  const request = parseRequest(line);
  if (typeof request === 'string') return served('-', refusal(request), trace);
  if (request.verb === 'approve-route' || request.verb === 'renew') trace.node = request.node;
  try {
    const policy = await readPolicyView(files.policy);
    const value = await answer(request, { runner, files, now, trace }, policy);
    return served(request.verb, { ok: true, value }, trace);
  } catch (error) {
    const reply = error instanceof Refused ? refusal(error.refusal, error.message) : refusal('headscale-failed');
    return served(request.verb, reply, trace);
  }
}
