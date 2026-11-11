/**
 * The `register` verb (vpn-plan.md Section 5.1, VP-7, VPN-003): `auth register` for the
 * operator user, then the audit of the node it made. The request saved the node ids before the
 * step (`pendingRegister`), so a step cut off after Headscale registered the node still leads to
 * an audit, here or in the next run (H1).
 */

import type { ListedNode } from '../src/inventory.ts';
import { OPERATOR, registered, registeredIdOf, type BrokerTailnet, type Registered } from './nodes.ts';
import { DETAILS, Refused } from './protocol.ts';
import { settle, type Run } from './settle.ts';
import { writeState, type BrokerState } from './state.ts';
import type { Step } from './verbs.ts';

/** The reply for one node of the tailnet. */
export function nodeReply(tailnet: BrokerTailnet, id: number): Registered {
  const node = tailnet.nodes.find((candidate) => candidate.id === id);
  if (node === undefined) throw new Refused('headscale-failed', 'the node is not in the node list');
  return registered(node);
}

/**
 * Whether `auth register` returned a node that was there before and that the broker may not
 * take over (L2): a tagged node, another user's node, or one it could not read.
 */
function foreign(id: number, before: BrokerTailnet): boolean {
  if (before.unreadable.includes(id)) return true;
  const node: ListedNode | undefined = before.nodes.find((candidate) => candidate.id === id);
  return node !== undefined && (node.tags.length > 0 || node.user !== OPERATOR);
}

async function step(run: Run, state: BrokerState, register: Step): Promise<number> {
  try {
    return registeredIdOf(JSON.parse(await run.runner(register)));
  } catch (error) {
    // The step may have registered a node before it was cut off: audit every new node now.
    // If this audit fails too, `pendingRegister` stays and the next run audits them.
    await settle(run, state);
    throw error;
  }
}

/**
 * Runs `auth register` and audits its node. `state.pendingRegister` holds the node ids of
 * `before`, written before this call. The node id comes from the reply, never from a difference
 * of two lists (another node may join).
 */
export async function register(run: Run, state: BrokerState, registerStep: Step, before: BrokerTailnet): Promise<Registered> {
  const id = await step(run, state, registerStep);
  run.trace.node = id;
  if (foreign(id, before)) {
    // Left as it was: deleting it would let an auth ID remove a router (L2).
    await settle(run, state);
    throw new Refused('not-allowed', 'the auth ID belongs to a tagged node or to another user; the node was left as it was');
  }
  const pending: BrokerState = { ...state, pendingRegister: { before: state.pendingRegister?.before ?? [], node: id } };
  await writeState(run.files.state, pending);
  const settled = await settle(run, pending);
  const undone = settled.fixes.find((fix) => fix.node === id && fix.kind === 'delete');
  if (undone?.kind === 'delete') throw new Refused('undone', `${DETAILS.undone}: ${undone.reason}`);
  return nodeReply(settled.tailnet, id);
}
