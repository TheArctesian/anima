// The K7 static file headers of the web client (K7 "HTTP surface", SEC-001, D-2), as pure verdicts on the
// headers of one answer. A verdict is `undefined` when the rule holds, else a fixed reason: it may name a header
// or a CSP directive K7 lists, never a value the hub sent.

/** The CSP directives whose sources K7 fixes, in its order; `script-src` and `connect-src` are checked apart. */
const FIXED_DIRECTIVES: Readonly<Record<string, readonly string[]>> = {
  "default-src": ["'self'"],
  "img-src": ["'self'", "data:", "blob:"],
  "style-src": ["'self'", "'unsafe-inline'"],
  "font-src": ["'self'"],
  "object-src": ["'none'"],
  "base-uri": ["'none'"],
  "form-action": ["'none'"],
  "frame-ancestors": ["'none'"],
};
const DIRECTIVE_COUNT = Object.keys(FIXED_DIRECTIVES).length + 2;
const SCRIPT_HASH = /^'sha(?:256|384|512)-[A-Za-z0-9+/]+={0,2}'$/;

/** The headers whose value K7 fixes on every web client answer (the frame rule is CSP `frame-ancestors`). */
const FIXED_HEADERS: readonly [string, string][] = [
  ["x-content-type-options", "nosniff"],
  ["referrer-policy", "no-referrer"],
  ["cross-origin-opener-policy", "same-origin"],
  ["strict-transport-security", "max-age=31536000"],
];

/** `Cache-Control` for `index.html` and for a file under `/_app/immutable/`. */
export const NO_CACHE = "no-cache";
export const IMMUTABLE = "public, max-age=31536000, immutable";

/** The WebSocket form of an origin, as the hub writes it into `connect-src` (`wss://` for `https://`). */
function websocketOrigin(origin: URL): string {
  return `${origin.protocol === "https:" ? "wss:" : "ws:"}//${origin.host}`;
}

// CSP splits on ASCII white space only, so a non-breaking space cannot hide a source inside a token (S-C5).
const ASCII_SPACE = /[\t\n\f\r ]+/;
const DIRECTIVE_NAME = /^[A-Za-z0-9-]+$/;

/** The directives of a policy by name, or the reason it cannot be read: a name repeats or is malformed. */
function directives(policy: string): Map<string, string[]> | string {
  const found = new Map<string, string[]>();
  for (const part of policy.split(";")) {
    const [name, ...sources] = part.split(ASCII_SPACE).filter((token) => token !== "");
    if (name === undefined) {
      continue;
    }
    if (!DIRECTIVE_NAME.test(name)) {
      return "the CSP has a malformed directive name";
    }
    if (found.has(name.toLowerCase())) {
      return "the CSP names a directive twice";
    }
    found.set(name.toLowerCase(), sources);
  }
  return found;
}

function same(actual: readonly string[] | undefined, expected: readonly string[]): boolean {
  return actual?.length === expected.length && expected.every((source, index) => actual[index] === source);
}

/** `script-src 'self'` and one or more script hashes, nothing else. */
function scriptsVerdict(sources: readonly string[] | undefined): string | undefined {
  if (sources?.[0] !== "'self'" || sources.length < 2) {
    return "the CSP directive script-src is not 'self' and the page's script hashes";
  }
  return sources.slice(1).every((source) => SCRIPT_HASH.test(source))
    ? undefined
    : "the CSP directive script-src allows a source that is not a script hash";
}

/** The `Content-Security-Policy` of a web client answer is the K7 policy for the hub at `origin`. */
export function policyVerdict(headers: Headers, origin: URL): string | undefined {
  const policy = headers.get("content-security-policy");
  if (policy === null) {
    return "the content-security-policy header is missing";
  }
  const found = directives(policy);
  if (typeof found === "string") {
    return found;
  }
  for (const [name, sources] of Object.entries(FIXED_DIRECTIVES)) {
    if (!same(found.get(name), sources)) {
      return `the CSP directive ${name} is not as K7 states`;
    }
  }
  if (!same(found.get("connect-src"), ["'self'", websocketOrigin(origin)])) {
    return "the CSP directive connect-src is not 'self' and the hub's WebSocket origin";
  }
  const scripts = scriptsVerdict(found.get("script-src"));
  if (scripts !== undefined) {
    return scripts;
  }
  return found.size === DIRECTIVE_COUNT ? undefined : "the CSP has a directive K7 does not list";
}

/** The fixed K7 headers are each present once with their K7 value. */
export function fixedHeadersVerdict(headers: Headers): string | undefined {
  for (const [name, value] of FIXED_HEADERS) {
    const actual = headers.get(name);
    if (actual?.trim().toLowerCase() !== value) {
      return actual === null ? `the ${name} header is missing` : `the ${name} header is not ${value}`;
    }
  }
  return undefined;
}

/** `Cache-Control` is exactly `expected`. */
export function cacheVerdict(headers: Headers, expected: string): string | undefined {
  const actual = headers.get("cache-control");
  if (actual === null) {
    return "the cache-control header is missing";
  }
  return actual.trim().toLowerCase() === expected ? undefined : `the cache-control header is not ${expected}`;
}

// A path the page names under `_app/immutable/`, relative or from the root; only path characters, so the probe
// never leaves the hub's origin.
const ASSET_REFERENCE = /(?:^|["'(=\s,])((?:\.{1,2}\/|\/)*_app\/immutable\/[A-Za-z0-9._\-/]+\.(?:js|css|woff2?))\b/;

/** The root path of the first hashed asset `index.html` names, or undefined when it names none. */
export function immutableAssetPath(html: string): string | undefined {
  const reference = ASSET_REFERENCE.exec(html)?.[1];
  if (reference === undefined) {
    return undefined;
  }
  const path = `/${reference.replace(/^(?:\.{1,2}\/|\/)*/, "")}`;
  return path.split("/").some((segment) => segment === "." || segment === "..") ? undefined : path;
}
