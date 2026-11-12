/**
 * Makes a captured Headscale or Tailscale JSON file safe to commit (vpn-build.md rule 0.6,
 * LOG-007). Pure. Node N gets `nodekey:`, `mkey:` and `discokey:` followed by N as 64 hex digits,
 * everywhere its keys appear (values and map keys). A pre-auth key becomes `REDACTED-PREAUTH-KEY`.
 * Anything that still looks like a key afterwards is an error: the capture fails, nothing leaks.
 */

/** Node N is the position of its host name here; the harness joins its containers in this order. */
export const NODE_NAMES = ['hub', 'router-home', 'mac', 'stranger', 'odd', 'mac2'] as const;
export const PREAUTH_PLACEHOLDER = 'REDACTED-PREAUTH-KEY';

export class RedactError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'RedactError';
  }
}

const KEY = /^(nodekey|mkey|discokey|privkey):[0-9a-f]{64}$/;
const KEY_FIELDS = ['PublicKey', 'node_key', 'machine_key', 'disco_key'];
const PREAUTH = /^hskey-auth-/;
const HEX_DIGITS = 64;

type Json = null | boolean | number | string | readonly Json[] | { readonly [key: string]: Json };
type Tree = Record<string, Json>;

const isTree = (value: unknown): value is Tree => typeof value === 'object' && value !== null && !Array.isArray(value);

function numberOf(element: Tree): number {
  const name = element.HostName ?? element.name;
  const index = NODE_NAMES.findIndex((candidate) => candidate === name);
  if (index < 0) throw new RedactError(`a node with an unknown host name: ${String(name)}`);
  return index + 1;
}

// Each element that carries a key and a host name gives the real key a placeholder.
function placeholders(value: unknown, found = new Map<string, string>()): Map<string, string> {
  if (Array.isArray(value)) value.forEach((entry) => placeholders(entry, found));
  else if (isTree(value)) {
    const keys = KEY_FIELDS.map((field) => value[field]).filter((entry): entry is string => typeof entry === 'string' && KEY.test(entry));
    if (keys.length > 0) {
      const digits = numberOf(value).toString(16).padStart(HEX_DIGITS, '0');
      for (const key of keys) found.set(key, `${key.slice(0, key.indexOf(':'))}:${digits}`);
    }
    Object.values(value).forEach((entry) => placeholders(entry, found));
  }
  return found;
}

function replace(value: Json, found: ReadonlyMap<string, string>): Json {
  if (typeof value === 'string') {
    if (PREAUTH.test(value)) return PREAUTH_PLACEHOLDER;
    return found.get(value) ?? value;
  }
  if (Array.isArray(value)) return value.map((entry) => replace(entry as Json, found));
  if (isTree(value)) return Object.fromEntries(Object.entries(value).map(([key, entry]) => [found.get(key) ?? key, replace(entry, found)]));
  return value;
}

function assertClean(text: string, allowed: ReadonlySet<string>): void {
  const keys = text.match(/(?:nodekey|mkey|discokey|privkey):[0-9a-f]{64}/g) ?? [];
  if (keys.some((key) => !allowed.has(key))) throw new RedactError('a key is still in the output');
  if (/hskey-|\/register\//.test(text)) throw new RedactError('a pre-auth key or auth ID is still in the output');
}

/** The captured JSON text with every key replaced; throws RedactError when a secret remains. */
export function redact(text: string): string {
  const tree = JSON.parse(text) as Json;
  const found = placeholders(tree);
  const out = `${JSON.stringify(replace(tree, found), null, 2)}\n`;
  assertClean(out, new Set(found.values()));
  return out;
}
