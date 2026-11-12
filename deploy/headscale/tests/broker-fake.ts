/**
 * A fake Headscale 0.29.7 for the broker tests: a `runner` that keeps nodes in memory and answers
 * each step with the shapes the contract harness captured (nodes.json, routes.json,
 * preauthkey.json, register.json). Keys use the placeholder of vpn-build.md rule 0.6.
 */
import { copyFileSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import type { Runner } from '../broker/settle.ts';
import type { BrokerFiles } from '../broker/state.ts';
import type { Step } from '../broker/verbs.ts';

export const KEY = 'REDACTED-PREAUTH-KEY';
const DAY_SECONDS = 86_400;
const TAGGED = { id: 2_147_455_555, name: 'tagged-devices' };
const OPERATOR = { id: 1, name: 'operator' };

export interface FakeNode {
  id: number;
  name: string;
  tags: string[];
  expiry: number | null;
  preAuthKeyId: number | null;
  available: string[];
  approved: string[];
}

interface FakeKey {
  readonly id: number;
  readonly tags: readonly string[];
}

const hex = (n: number): string => n.toString(16).padStart(64, '0');
const seconds = (date: Date): number => Math.floor(date.getTime() / 1000);

function flag(step: Step, name: string): string | undefined {
  return step.find((arg) => arg.startsWith(`--${name}=`))?.slice(name.length + 3);
}

const folders: string[] = [];

/** A temporary broker.conf, the repository policy and a state path; `removeBrokerFiles` cleans up. */
export function brokerFiles(maxNodes = 10): BrokerFiles {
  const folder = mkdtempSync(join(tmpdir(), 'anima-broker-'));
  folders.push(folder);
  const files = { config: join(folder, 'broker.conf'), policy: join(folder, 'policy.hujson'), state: join(folder, 'state.json') };
  writeFileSync(files.config, `max_nodes=${String(maxNodes)}\n`);
  copyFileSync(new URL('../config/policy.hujson', import.meta.url), files.policy);
  return files;
}

/** Removes every folder `brokerFiles` made; call from `afterEach`. */
export function removeBrokerFiles(): void {
  for (const folder of folders.splice(0)) rmSync(folder, { recursive: true, force: true });
}

export class FakeHeadscale {
  readonly nodes: FakeNode[] = [];
  readonly steps: Step[] = [];
  readonly keys: FakeKey[] = [];
  /** Auth IDs of computers waiting for `auth register`, with what they will become. */
  readonly waiting = new Map<string, { name: string; tags: string[]; id?: number }>();
  /** Runs after a step changed the fake, before it answers (a kill, or another node joining). */
  afterStep: (step: Step) => void = () => undefined;
  now: Date;
  private nextId = 1;

  constructor(now: Date) {
    this.now = now;
  }

  readonly runner: Runner = (step) => {
    this.steps.push(step);
    const reply = this.answer(step);
    this.afterStep(step);
    return Promise.resolve(reply);
  };

  add(name: string, fields: Partial<FakeNode> = {}): FakeNode {
    const node: FakeNode = { id: this.nextId, name, tags: [], expiry: seconds(this.now) + 180 * DAY_SECONDS, preAuthKeyId: null, available: [], approved: [], ...fields };
    this.nextId = Math.max(this.nextId, node.id) + 1;
    this.nodes.push(node);
    return node;
  }

  /** A computer joins with the pre-auth key `keyId`; the key's tags apply unless `tags` is given. */
  join(keyId: number, name: string, fields: Partial<FakeNode> = {}): FakeNode {
    const key = this.keys.find((candidate) => candidate.id === keyId);
    if (key === undefined) throw new Error(`fake: no key ${String(keyId)}`);
    const tagged = key.tags.length > 0;
    return this.add(name, { tags: [...key.tags], expiry: tagged ? null : seconds(this.now) + 180 * DAY_SECONDS, preAuthKeyId: keyId, ...fields });
  }

  node(id: number): FakeNode | undefined {
    return this.nodes.find((candidate) => candidate.id === id);
  }

  ran(verb: string, action: string): Step[] {
    return this.steps.filter((step) => step[0] === verb && step[1] === action);
  }

  private raw(node: FakeNode): Record<string, unknown> {
    const tagged = node.tags.length > 0;
    return {
      id: node.id,
      machine_key: `mkey:${hex(node.id)}`,
      node_key: `nodekey:${hex(node.id)}`,
      disco_key: `discokey:${hex(node.id)}`,
      ip_addresses: [`100.64.0.${String(node.id)}`, `fd7a:115c:a1e0::${String(node.id)}`],
      name: node.name,
      user: tagged ? TAGGED : OPERATOR,
      ...(node.preAuthKeyId === null ? {} : { pre_auth_key: { id: node.preAuthKeyId, key: KEY, used: true } }),
      ...(node.expiry === null ? {} : { expiry: { seconds: node.expiry, nanos: 1 } }),
      ...(tagged ? { tags: node.tags } : {}),
      ...(node.approved.length > 0 ? { approved_routes: node.approved } : {}),
      ...(node.available.length > 0 ? { available_routes: node.available } : {}),
      online: true,
    };
  }

  private target(step: Step): FakeNode {
    const node = this.node(Number(flag(step, 'identifier')));
    if (node === undefined) throw new Error('fake: node not found');
    return node;
  }

  private createKey(step: Step): string {
    if (flag(step, 'expiration') !== '1h' || step.some((arg) => /reusable|ephemeral/.test(arg))) throw new Error('fake: unexpected key flags');
    const user = flag(step, 'user');
    if (user !== undefined && user !== String(OPERATOR.id)) throw new Error('fake: --user takes the numeric id');
    const key = { id: this.keys.length + 1, tags: flag(step, 'tags')?.split(',') ?? [] };
    this.keys.push(key);
    const expiration = { seconds: seconds(this.now) + 3600, nanos: 5 };
    return JSON.stringify({ id: key.id, key: KEY, expiration, created_at: { seconds: seconds(this.now) }, ...(key.tags.length > 0 ? { acl_tags: key.tags } : {}) });
  }

  private register(step: Step): string {
    const waiting = this.waiting.get(flag(step, 'auth-id') ?? '');
    if (flag(step, 'user') !== OPERATOR.name || waiting === undefined) throw new Error('fake: node not found in registration cache');
    this.waiting.delete(flag(step, 'auth-id') ?? '');
    const expiry = seconds(this.now) + 180 * DAY_SECONDS;
    const existing = waiting.id === undefined ? undefined : this.node(waiting.id);
    const node = existing ?? this.add(waiting.name, { tags: waiting.tags, expiry });
    node.expiry = node.tags.length > 0 ? null : expiry;
    // register.json: the node itself, with no pre_auth_key and no online key.
    const reply = Object.fromEntries(Object.entries(this.raw(node)).filter(([field]) => field !== 'pre_auth_key' && field !== 'online'));
    return JSON.stringify({ ...reply, register_method: 2 });
  }

  private answer(step: Step): string {
    const command = `${step[0] ?? ''} ${step[1] ?? ''}`;
    switch (command) {
      case 'nodes list':
        return JSON.stringify(this.nodes.length === 0 ? null : this.nodes.map((node) => this.raw(node)));
      case 'nodes list-routes':
        return JSON.stringify(this.nodes.filter((node) => node.available.length > 0).map((node) => this.raw(node)));
      case 'users list':
        return JSON.stringify([{ id: 2, name: 'intruder' }, { ...OPERATOR, created_at: { seconds: 1 } }]);
      case 'preauthkeys create':
        return this.createKey(step);
      case 'auth register':
        return this.register(step);
      case 'nodes approve-routes':
        this.target(step).approved = (flag(step, 'routes') ?? '').split(',');
        return '{}';
      case 'nodes expire': {
        const expiry = flag(step, 'expiry');
        if (expiry === undefined || expiry === '') throw new Error('fake: an empty --expiry expires the node at once');
        this.target(step).expiry = seconds(new Date(expiry));
        return '{}';
      }
      case 'nodes delete':
        this.nodes.splice(this.nodes.indexOf(this.target(step)), 1);
        return '';
      default:
        throw new Error(`fake: unknown step ${command}`);
    }
  }
}
