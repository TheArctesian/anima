import { fingerprintFrom, type Fingerprint } from './fingerprint';

/** What the shell reports about one tab's web view (WP-31 "Shell interface"). */
export type WebViewEvent =
  | { readonly tabId: string; readonly kind: 'LOADING' | 'LOADED'; readonly url: string }
  | { readonly tabId: string; readonly kind: 'UNTRUSTED'; readonly fingerprintSha256: Fingerprint }
  | { readonly tabId: string; readonly kind: 'MISMATCH'; readonly expected: Fingerprint; readonly presented: Fingerprint }
  | { readonly tabId: string; readonly kind: 'FAILED'; readonly reason: string };

/** A tab ID as the client makes it; it becomes part of the web view label `web-<tab id>` (D-3). */
const TAB_ID = /^[A-Za-z0-9_-]{1,128}$/;
/** A failure reason is a short code from the shell, never a URL or page text (LOG-007). */
const REASON = /^[a-z][a-z0-9_]{0,63}$/;
const URL_MAX = 8192;

const FIELDS = {
  LOADING: ['url'],
  LOADED: ['url'],
  UNTRUSTED: ['fingerprintSha256'],
  MISMATCH: ['expected', 'presented'],
  FAILED: ['reason'],
} as const;

type Kind = keyof typeof FIELDS;

function isKind(value: unknown): value is Kind {
  return typeof value === 'string' && Object.hasOwn(FIELDS, value);
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

/** Whether `value` has exactly the keys `tabId`, `kind` and `fields`, so an unknown shape is never half read. */
function hasExactly(value: Record<string, unknown>, fields: readonly string[]): boolean {
  const keys = Object.keys(value);
  const wanted = ['tabId', 'kind', ...fields];
  return keys.length === wanted.length && wanted.every((key) => Object.hasOwn(value, key));
}

/** `value` as an `http` or `https` URL the shell reported, or `null`. */
export function webUrlFrom(value: unknown): string | null {
  if (typeof value !== 'string' || value.length === 0 || value.length > URL_MAX) return null;
  if (!URL.canParse(value)) return null;
  const { protocol } = new URL(value);
  return protocol === 'http:' || protocol === 'https:' ? value : null;
}

function eventOf(tabId: string, kind: Kind, fields: Record<string, unknown>): WebViewEvent | null {
  switch (kind) {
    case 'LOADING':
    case 'LOADED': {
      const url = webUrlFrom(fields.url);
      return url === null ? null : { tabId, kind, url };
    }
    case 'UNTRUSTED': {
      const fingerprintSha256 = fingerprintFrom(fields.fingerprintSha256);
      return fingerprintSha256 === null ? null : { tabId, kind, fingerprintSha256 };
    }
    case 'MISMATCH': {
      const expected = fingerprintFrom(fields.expected);
      const presented = fingerprintFrom(fields.presented);
      return expected === null || presented === null ? null : { tabId, kind, expected, presented };
    }
    case 'FAILED': {
      const { reason } = fields;
      return typeof reason === 'string' && REASON.test(reason) ? { tabId, kind, reason } : null;
    }
  }
}

/**
 * The `web-view` event payload as a `WebViewEvent`, or `null` for any other shape. The payload crosses a process
 * boundary, so every field is checked and an extra field drops the event.
 */
export function webViewEventFrom(payload: unknown): WebViewEvent | null {
  if (!isRecord(payload)) return null;
  const { tabId, kind } = payload;
  if (typeof tabId !== 'string' || !TAB_ID.test(tabId) || !isKind(kind)) return null;
  if (!hasExactly(payload, FIELDS[kind])) return null;
  return eventOf(tabId, kind, payload);
}
