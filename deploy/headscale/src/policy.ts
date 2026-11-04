/**
 * Anima's Headscale grants policy (VPN-004), parsed into resolved selectors. Pure. Only
 * the subset of the format Anima uses is accepted; anything else is a `PolicyError`, so
 * the model never silently disagrees with the server about a rule it does not understand.
 */

export type PolicyErrorCode =
  | 'invalid-shape'
  | 'unsupported-section'
  | 'unsupported-selector'
  | 'unknown-selector'
  | 'unknown-tag'
  | 'invalid-address'
  | 'invalid-port'
  | 'invalid-protocol'
  | 'invalid-node'
  | 'unknown-node';

export class PolicyError extends Error {
  readonly code: PolicyErrorCode;

  constructor(code: PolicyErrorCode, message: string) {
    super(`${code}: ${message}`);
    this.name = 'PolicyError';
    this.code = code;
  }
}

interface PolicyTest {
  readonly src: string;
  /** Each row must hold for every protocol listed here. */
  readonly protos: readonly string[];
  readonly accept: readonly string[];
  readonly deny: readonly string[];
}

export interface Policy {
  readonly grants: readonly Grant[];
  readonly tests: readonly PolicyTest[];
  readonly scope: Scope;
}

export interface Prefix {
  readonly base: number;
  readonly bits: number;
}

export type Selector =
  | { readonly kind: 'users'; readonly names: readonly string[] }
  | { readonly kind: 'tag'; readonly tag: string }
  | { readonly kind: 'prefix'; readonly prefix: Prefix };

export interface PortRule {
  readonly proto: string;
  readonly first: number;
  readonly last: number;
}

interface Grant {
  readonly src: readonly Selector[];
  readonly dst: readonly Selector[];
  readonly ip: readonly PortRule[];
}

interface Scope {
  readonly hosts: ReadonlyMap<string, Prefix>;
  readonly tags: ReadonlySet<string>;
}

const SECTIONS = ['tagOwners', 'hosts', 'grants', 'tests'];
// A tests row without `proto` is checked by Headscale for each of these.
const TEST_PROTOCOLS = ['tcp', 'udp', 'icmp'];
const PROTOCOLS = new Set(['tcp', 'udp', 'sctp', 'icmp']);
const OCTET = /^(?:0|[1-9]\d{0,2})$/;
const PORT = /^\d{1,5}$/;
const HOST_NAME = /^[a-z][a-z0-9-]*$/i;
export const MAX_PORT = 65_535;

export function parsePolicy(raw: unknown): Policy {
  const root = record(raw, 'policy');
  onlyKeys(root, SECTIONS, 'policy');
  // A missing grants list makes Headscale allow everything; an empty one denies
  // everything. Both are refused because Anima requires the D-4 grants (VPN-004).
  const grants = list(root.grants, 'grants');
  if (grants.length === 0) throw new PolicyError('invalid-shape', 'grants is empty; Anima requires the D-4 grants');
  const scope = parseScope(root);
  return {
    grants: grants.map((grant, index) => parseGrant(grant, scope, `grants[${String(index)}]`)),
    tests: list(root.tests ?? [], 'tests').map((test, index) => parseTest(test, scope, `tests[${String(index)}]`)),
    scope,
  };
}

function parseScope(root: Readonly<Record<string, unknown>>): Scope {
  const tagOwners = Object.entries(record(root.tagOwners ?? {}, 'tagOwners'));
  const tags = new Set(tagOwners.map(([tag]) => tag));
  if (![...tags].every((tag) => /^tag:[a-z0-9-]+$/i.test(tag))) throw new PolicyError('invalid-shape', 'tagOwners keys must be tag:<name>');
  const hosts = new Map(
    Object.entries(record(root.hosts ?? {}, 'hosts')).map(([name, value]) => {
      if (!HOST_NAME.test(name)) throw new PolicyError('invalid-shape', `host alias ${name} is not a plain name`);
      if (typeof value !== 'string') throw new PolicyError('invalid-shape', `hosts.${name} must be a string`);
      return [name, parsePrefix(value)] as const;
    }),
  );
  const scope = { hosts, tags };
  for (const [tag, owners] of tagOwners) {
    for (const owner of stringList(owners, tag)) {
      if (parseSelector(owner, scope).kind === 'prefix') throw new PolicyError('invalid-shape', `${tag} owner ${owner} is an address`);
    }
  }
  return scope;
}

function userName(selector: string): string {
  return selector.endsWith('@') ? selector.slice(0, -1) : selector;
}

function parseGrant(value: unknown, scope: Scope, where: string): Grant {
  const grant = record(value, where);
  onlyKeys(grant, ['src', 'dst', 'ip'], where);
  const field = (key: string): string[] => {
    const entries = stringList(grant[key], `${where}.${key}`);
    if (entries.length === 0) throw new PolicyError('invalid-shape', `${where}.${key} is empty`);
    return entries;
  };
  return {
    src: field('src').map((selector) => parseSelector(selector, scope)),
    dst: field('dst').map((selector) => parseSelector(selector, scope)),
    ip: field('ip').map(parsePortRule),
  };
}

function parseTest(value: unknown, scope: Scope, where: string): PolicyTest {
  const test = record(value, where);
  onlyKeys(test, ['src', 'proto', 'accept', 'deny'], where);
  if (typeof test.src !== 'string') throw new PolicyError('invalid-shape', `${where}.src must be a string`);
  const accept = stringList(test.accept ?? [], `${where}.accept`);
  const deny = stringList(test.deny ?? [], `${where}.deny`);
  if (accept.length + deny.length === 0) throw new PolicyError('invalid-shape', `${where} checks nothing`);
  parseSelector(test.src, scope);
  for (const dst of [...accept, ...deny]) parseSelector(parseTarget(dst).host, scope);
  const protos = test.proto === undefined ? TEST_PROTOCOLS : [checkProto(test.proto)];
  return { src: test.src, protos, accept, deny };
}

export function parseSelector(text: string, scope: Scope): Selector {
  if (text === '*' || text.startsWith('autogroup:') || text.startsWith('group:')) {
    throw new PolicyError('unsupported-selector', `${text} is not allowed; name users, tags or sites (VPN-004)`);
  }
  if (text.startsWith('tag:')) {
    if (!scope.tags.has(text)) throw new PolicyError('unknown-tag', `${text} is not in tagOwners`);
    return { kind: 'tag', tag: text };
  }
  if (text.includes('@')) return { kind: 'users', names: [userName(text)] };
  const host = scope.hosts.get(text);
  if (host !== undefined) return { kind: 'prefix', prefix: host };
  if (/^[\d./]+$/.test(text)) return { kind: 'prefix', prefix: parsePrefix(text) };
  if (text.includes(':')) throw new PolicyError('invalid-address', `${text}: only IPv4 is modelled`);
  throw new PolicyError('unknown-selector', text);
}

export function parseTarget(text: string): { host: string; port: number } {
  const colon = text.lastIndexOf(':');
  if (colon <= 0) throw new PolicyError('invalid-shape', `test destination ${text} must be host:port`);
  return { host: text.slice(0, colon), port: parsePortNumber(text.slice(colon + 1), text) };
}

function parsePortRule(text: string): PortRule {
  if (text === '*') return { proto: '*', first: 0, last: MAX_PORT };
  const colon = text.indexOf(':');
  const proto = colon === -1 ? '*' : checkProto(text.slice(0, colon));
  const ports = text.slice(colon + 1);
  if (ports === '*') return { proto, first: 0, last: MAX_PORT };
  const bounds = ports.split('-');
  if (bounds.length > 2) throw new PolicyError('invalid-port', text);
  const first = parsePortNumber(bounds[0] ?? '', text);
  const last = parsePortNumber(bounds[1] ?? bounds[0] ?? '', text);
  if (first > last) throw new PolicyError('invalid-port', `${text}: range runs backwards`);
  return { proto, first, last };
}

function parsePortNumber(text: string, context: string): number {
  if (!PORT.test(text)) throw new PolicyError('invalid-port', context);
  return checkPort(Number(text));
}

export function checkPort(port: number): number {
  if (!Number.isInteger(port) || port < 0 || port > MAX_PORT) throw new PolicyError('invalid-port', String(port));
  return port;
}

export function checkProto(proto: unknown): string {
  if (typeof proto !== 'string' || !PROTOCOLS.has(proto)) throw new PolicyError('invalid-protocol', `unknown protocol ${String(proto)}`);
  return proto;
}

export function parsePrefix(text: string): Prefix {
  const parts = text.split('/');
  if (parts.length > 2) throw new PolicyError('invalid-address', text);
  const base = parseIpv4(parts[0] ?? '');
  const bitsText = parts[1] ?? '32';
  const bits = Number(bitsText);
  if (!/^\d{1,2}$/.test(bitsText) || bits > 32) throw new PolicyError('invalid-address', text);
  // A /0 prefix is the wildcard under another name (VPN-004).
  if (bits === 0) throw new PolicyError('unsupported-selector', `${text} matches every address`);
  if (base % 2 ** (32 - bits) !== 0) throw new PolicyError('invalid-address', `${text} has host bits set`);
  return { base, bits };
}

export function parseIpv4(text: string): number {
  const octets = text.split('.');
  if (octets.length !== 4 || !octets.every((octet) => OCTET.test(octet) && Number(octet) <= 255)) {
    throw new PolicyError('invalid-address', `${text} is not an IPv4 address`);
  }
  return octets.reduce((sum, octet) => sum * 256 + Number(octet), 0);
}

export function contains(prefix: Prefix, ip: number): boolean {
  const size = 2 ** (32 - prefix.bits);
  return Math.floor(ip / size) === prefix.base / size;
}

function record(value: unknown, where: string): Readonly<Record<string, unknown>> {
  if (typeof value !== 'object' || value === null || Array.isArray(value)) throw new PolicyError('invalid-shape', `${where} must be an object`);
  return value as Record<string, unknown>;
}

function list(value: unknown, where: string): readonly unknown[] {
  if (!Array.isArray(value)) throw new PolicyError('invalid-shape', `${where} must be a list`);
  return value as unknown[];
}

function stringList(value: unknown, where: string): string[] {
  const entries = list(value, where);
  if (!entries.every((entry) => typeof entry === 'string')) throw new PolicyError('invalid-shape', `${where} must list strings`);
  return entries as string[];
}

function onlyKeys(value: Readonly<Record<string, unknown>>, allowed: readonly string[], where: string): void {
  const extra = Object.keys(value).find((key) => !allowed.includes(key));
  if (extra !== undefined) throw new PolicyError('unsupported-section', `${where}.${extra} is not used by Anima (VPN-004)`);
}
