/**
 * The broker's wire format (R-055, VP-7, VP-10): one JSON request line in, one JSON reply
 * line out. Pure. The request comes from the hub's user, so every byte is hostile: anything
 * that is not exactly one of the six verbs with valid values is `bad-request`.
 */

export type Request =
  | { readonly verb: 'nodes' }
  | { readonly verb: 'personal-key' }
  | { readonly verb: 'router-key'; readonly site: string }
  | { readonly verb: 'approve-route'; readonly node: number; readonly site: string }
  | { readonly verb: 'register'; readonly authId: string }
  | { readonly verb: 'renew'; readonly node: number };

export type Verb = Request['verb'];

export type Refusal = 'bad-request' | 'unknown-site' | 'rate-limited' | 'node-limit' | 'not-allowed' | 'undone' | 'headscale-failed';

export type Reply<T> = { readonly ok: true; readonly value: T } | { readonly ok: false; readonly refusal: Refusal; readonly detail: string };

/** The longest request line, in UTF-8 bytes, without the newline. */
export const MAX_REQUEST_BYTES = 1024;

const SITE = /^[a-z0-9-]{1,32}$/;
// Headscale 0.29.7 auth IDs: the prefix and 24 base64url characters (vpn-build.md Section 3, fact 11).
const AUTH_ID = /^hskey-authreq-[A-Za-z0-9_-]{24}$/;
const NODE_LIMIT = 2 ** 31;

type Field = 'site' | 'node' | 'authId';

const FIELDS: Readonly<Record<Verb, readonly Field[]>> = {
  nodes: [],
  'personal-key': [],
  'router-key': ['site'],
  'approve-route': ['node', 'site'],
  register: ['authId'],
  renew: ['node'],
};

const VALID: Readonly<Record<Field, (value: unknown) => boolean>> = {
  site: (value) => typeof value === 'string' && SITE.test(value),
  node: (value) => typeof value === 'number' && Number.isSafeInteger(value) && value > 0 && value < NODE_LIMIT,
  authId: (value) => typeof value === 'string' && AUTH_ID.test(value),
};

function isVerb(value: unknown): value is Verb {
  return typeof value === 'string' && Object.hasOwn(FIELDS, value);
}

function decode(line: string): Readonly<Record<string, unknown>> | null {
  if (new TextEncoder().encode(line).length > MAX_REQUEST_BYTES) return null;
  let value: unknown;
  try {
    value = JSON.parse(line);
  } catch {
    return null;
  }
  if (typeof value !== 'object' || value === null || Array.isArray(value)) return null;
  return value as Readonly<Record<string, unknown>>;
}

/** Parses one request line; `bad-request` for anything else. Max 1 KiB. */
export function parseRequest(line: string): Request | Refusal {
  const body = decode(line);
  if (body === null || !Object.hasOwn(body, 'verb') || !isVerb(body.verb)) return 'bad-request';
  const fields = FIELDS[body.verb];
  const keys = Object.keys(body);
  if (keys.length !== fields.length + 1) return 'bad-request';
  for (const field of fields) {
    if (!Object.hasOwn(body, field) || !VALID[field](body[field])) return 'bad-request';
  }
  const request: Record<string, unknown> = { verb: body.verb };
  for (const field of fields) request[field] = body[field];
  return request as Request;
}
