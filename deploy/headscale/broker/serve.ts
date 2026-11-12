/**
 * One broker request, from the request line to the reply (R-055, VP-7, VP-8): check the
 * nodes the broker is responsible for, check the limits, run the verb, check its result.
 * Headscale is reached only through `runner`, so tests pass a fake. The pre-auth key goes
 * only into the reply; the log line holds the verb, the outcome, a node id and counts.
 */

import { readFile } from 'node:fs/promises';
import { admit, record } from './limits.ts';
import { newKeyOf, nodeCountOf, nodeIdsOf, nodesReply, operatorIdOf, type BrokerTailnet } from './nodes.ts';
import type { Issued } from './outcome.ts';
import { DETAILS, parseRequest, Refused, type Refusal, type Reply, type Request, type Verb } from './protocol.ts';
import { nodeReply, register } from './register.ts';
import { json, logLine, newTrace, readTailnet, settle, type Run, type Runner, type Trace } from './settle.ts';
import { maxNodesOf, readPolicyView, readState, writeState, type BrokerFiles, type BrokerState } from './state.ts';
import { plan, USERS_STEP, type PolicyView, type Step } from './verbs.ts';

export interface Served {
  readonly reply: Reply<unknown>;
  /** The journald line: never a key, an auth ID or a node name (LOG-007). */
  readonly log: string;
}

const PER_DAY = 3;

function only(steps: readonly Step[]): Step {
  const [step] = steps;
  if (step === undefined || steps.length !== 1) throw new Refused('headscale-failed');
  return step;
}

async function newKey(run: Run, request: Request & { verb: 'personal-key' | 'router-key' }, state: BrokerState, steps: readonly Step[]): Promise<unknown> {
  const key = newKeyOf(JSON.parse(await run.runner(only(steps))));
  const entry: Issued = { keyId: key.id, verb: request.verb, ...(request.verb === 'router-key' ? { site: request.site } : {}), at: run.now.toISOString(), passed: false };
  await writeState(run.files.state, { ...state, issued: [...state.issued, entry] });
  return { key: key.key, expiresAt: key.expiresAt.toISOString(), keyId: key.id };
}

async function perform(run: Run, request: Request, state: BrokerState, planned: { steps: readonly Step[]; tailnet: BrokerTailnet }): Promise<unknown> {
  switch (request.verb) {
    case 'nodes':
      // Answered in `answer` from the list it already read.
      throw new Refused('headscale-failed');
    case 'personal-key':
    case 'router-key':
      return newKey(run, request, state, planned.steps);
    case 'approve-route':
      await run.runner(only(planned.steps));
      return { node: request.node, site: request.site };
    case 'register':
      return register(run, state, only(planned.steps), planned.tailnet);
    case 'renew':
      await run.runner(only(planned.steps));
      return nodeReply(await readTailnet(run), request.node);
  }
}

/** The state to write before the step: the count, and for `register` the node ids before it (H1). */
function beforeStep(request: Request, state: BrokerState, tailnet: BrokerTailnet, now: Date): BrokerState {
  const counted = { ...state, ...record(request.verb, state, now) };
  return request.verb === 'register' ? { ...counted, pendingRegister: { before: nodeIdsOf(tailnet), node: null } } : counted;
}

async function answer(request: Request, run: Run, policy: PolicyView): Promise<unknown> {
  const maxNodes = maxNodesOf(await readFile(run.files.config, 'utf8'));
  // A request killed during `auth register` left its nodes unchecked: check them first.
  const settled = await settle(run, await readState(run.files.state));
  if (request.verb === 'nodes') return nodesReply(settled.tailnet, policy);
  const operatorId = request.verb === 'personal-key' ? operatorIdOf(await json(run, USERS_STEP)) : null;
  const steps = plan(request, policy, nodesReply(settled.tailnet, policy).nodes, { now: run.now, operatorId });
  if (typeof steps === 'string') throw new Refused(steps);
  const usedKeys = new Set(settled.tailnet.nodes.flatMap((node) => (node.preAuthKeyId === null ? [] : [node.preAuthKeyId])));
  const usage = { state: settled.state, nodeCount: nodeCountOf(settled.tailnet), issued: settled.state.issued, usedKeys };
  const verdict = admit(request.verb, usage, { perDay: PER_DAY, maxNodes }, run.now);
  if (verdict !== 'ok') throw new Refused(verdict);
  // Written before the step, so a kill after the step cannot leave a key uncounted or a node unaudited.
  const counted = beforeStep(request, settled.state, settled.tailnet, run.now);
  await writeState(run.files.state, counted);
  return perform(run, request, counted, { steps, tailnet: settled.tailnet });
}

function served(verb: Verb | '-', reply: Reply<unknown>, trace: Trace): Served {
  return { reply, log: logLine(verb, reply.ok ? 'ok' : reply.refusal, trace) };
}

const refusal = (refused: Refusal, detail = DETAILS[refused]): Reply<unknown> => ({ ok: false, refusal: refused, detail });

/** Answers one request line. Every failure is a refusal; nothing here throws. */
export async function serve(line: string, runner: Runner, files: BrokerFiles, now: Date): Promise<Served> {
  const trace = newTrace();
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
