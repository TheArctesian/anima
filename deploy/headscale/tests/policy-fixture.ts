import { parseHujson } from '../src/hujson.ts';
import { parsePolicy, type Policy } from '../src/policy.ts';
import { allows, type Connection, type TailnetNode } from '../src/tailnet.ts';

export const policyText = await Bun.file(new URL('../config/policy.hujson', import.meta.url)).text();
export const raw = parseHujson(policyText) as Record<string, unknown>;
export const policy = parsePolicy(raw);

export const SITES = { home: '10.10.0.0/24', office1: '10.20.0.0/24', office2: '10.30.0.0/24' };
export const TAILNET_IPV4 = '100.64.0.0/10';
export const nodes: TailnetNode[] = [
  // The Mac the operator added as a device (R-061): `device-stephens-mac` in the policy.
  { name: 'mac', identity: { user: 'operator' }, ip: '100.64.0.2' },
  { name: 'laptop', identity: { user: 'operator' }, ip: '100.64.0.11' },
  { name: 'hub', identity: { tags: ['tag:hub'] }, ip: '100.64.0.1' },
  // An agent-only device (MET-020): tagged, not a personal node.
  { name: 'agent', identity: { tags: ['tag:agent'] }, ip: '100.64.0.30' },
  { name: 'router-home', identity: { tags: ['tag:router', 'tag:site-home'] }, ip: '100.64.0.20', routes: [SITES.home] },
  { name: 'router-office2', identity: { tags: ['tag:router', 'tag:site-office2'] }, ip: '100.64.0.21', routes: [SITES.office2] },
];

export type From = Connection['from'];
export type To = Connection['to'];
export const tcp = (from: From, to: To, port: number): Connection => ({ from, to, port });
export const allowed = (connection: Connection, under: Policy = policy, inventory = nodes): boolean => allows(under, inventory, connection);
export const withGrants = (grants: unknown[]): Policy => parsePolicy({ ...raw, grants });
export const withTests = (tests: unknown[]): Policy => parsePolicy({ ...raw, tests });

/**
 * Test rows that need a router node. Headscale refuses the policy while a row names a tag no node
 * has, so each group moves into the policy's tests block once its router is enrolled.
 */
export const ROUTER_ROWS = [
  // The home router (tag:router, tag:site-home).
  { src: 'tag:site-home', proto: 'tcp', accept: ['10.10.0.5:22'], deny: ['10.20.0.5:22', '10.30.0.5:22', 'operator@:22', 'tag:hub:443'] },
  { src: '10.10.0.5', deny: ['operator@:22', 'tag:hub:443', '10.30.0.5:22'] },
  { src: 'tag:router', proto: 'tcp', deny: ['operator@:22', 'operator@:443', 'tag:hub:443', 'tag:hub:22'] },
  { src: 'operator@', proto: 'tcp', deny: ['tag:router:22'] },
  { src: 'tag:hub', proto: 'tcp', deny: ['tag:router:22'] },
  // The office 2 router (tag:router, tag:site-office2).
  { src: 'tag:site-office2', proto: 'tcp', accept: ['10.30.0.5:22'], deny: ['10.10.0.5:22', '10.20.0.5:22', 'operator@:22', 'tag:hub:443'] },
  { src: '10.30.0.5', deny: ['operator@:22', 'tag:hub:443', '10.10.0.5:22'] },
];

/** Rows for the agent tag. Headscale refuses the policy while no node carries tag:agent, so they move into the policy's tests block once one is enrolled. */
export const AGENT_ROWS = [
  { src: 'tag:agent', proto: 'tcp', accept: ['tag:hub:443'], deny: ['tag:hub:22', 'tag:hub:80', 'tag:hub:5432', '10.10.0.5:22', '10.20.0.5:443', '10.30.0.5:3389', '100.64.0.2:22', '100.64.0.3:5900', 'operator@:443'] },
  { src: 'tag:agent', proto: 'udp', deny: ['tag:hub:443', 'tag:hub:53'] },
  { src: 'tag:hub', proto: 'tcp', deny: ['tag:agent:22', 'tag:agent:443'] },
  { src: 'operator@', proto: 'tcp', deny: ['tag:agent:22', 'tag:agent:443'] },
];

// A second agent node, so the tests show that one agent cannot reach another (agent-to-agent isolation, MET-020).
export const secondAgent: TailnetNode = { name: 'agent2', identity: { tags: ['tag:agent'] }, ip: '100.64.0.31' };
export const everyone = [...nodes, secondAgent];
