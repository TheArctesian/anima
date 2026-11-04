/** The K7 `extensions.code` values the hub sends. */
const HUB_CODES = ['UNAUTHENTICATED', 'FORBIDDEN', 'NOT_FOUND', 'VALIDATION', 'CONFLICT', 'UNAVAILABLE', 'INTERNAL'] as const;

export type HubCode = (typeof HUB_CODES)[number];

/** One rejected input field of a `VALIDATION` error. */
export interface FieldError {
  readonly field: string;
  readonly message: string;
}

/**
 * Why a hub call has no value. K7 codes come from the hub; `unreachable` means no answer arrived
 * (network failure, NET-004); `malformed` means an answer arrived that is not a GraphQL response.
 * `message` is a plain sentence for the operator and never holds the token (LOG-007).
 */
export type HubError =
  | { readonly code: Exclude<HubCode, 'VALIDATION'>; readonly message: string }
  | { readonly code: 'VALIDATION'; readonly message: string; readonly fields: readonly FieldError[] }
  | { readonly code: 'unreachable' | 'malformed'; readonly message: string };

/** The outcome of a hub call: a value, or the typed reason there is none. */
export type HubResult<T> = { readonly ok: true; readonly value: T } | { readonly ok: false; readonly error: HubError };

export const UNREACHABLE: HubError = {
  code: 'unreachable',
  message: 'The hub did not answer. Check the VPN client.',
};

export const MALFORMED: HubError = {
  code: 'malformed',
  message: 'The hub sent an answer that Anima cannot read.',
};

/** The token provider failed; nothing from its error is kept, since it may describe the key store (LOG-007). */
export const TOKEN_UNREADABLE: HubError = {
  code: 'UNAUTHENTICATED',
  message: 'The access token could not be read.',
};

const UNCODED_MESSAGE = 'The hub could not run the request.';

type JsonObject = Readonly<Record<string, unknown>>;

function isObject(value: unknown): value is JsonObject {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function isHubCode(value: unknown): value is HubCode {
  return HUB_CODES.some((code) => code === value);
}

function parseFields(value: unknown): FieldError[] | undefined {
  if (value === undefined) return [];
  if (!Array.isArray(value)) return undefined;
  const fields = value.flatMap((entry: unknown): FieldError[] =>
    isObject(entry) && typeof entry.field === 'string' && typeof entry.message === 'string'
      ? [{ field: entry.field, message: entry.message }]
      : [],
  );
  return fields.length === value.length ? fields : undefined;
}

/** One GraphQL error object as a typed error. An error without a K7 code (for example a query the hub refused to parse) is `INTERNAL`. */
function parseError(value: unknown): HubError {
  if (!isObject(value) || typeof value.message !== 'string') return MALFORMED;
  const message = value.message;
  const extensions = isObject(value.extensions) ? value.extensions : {};
  const code = extensions.code;
  if (code === 'VALIDATION') {
    const fields = parseFields(extensions.fields);
    return fields === undefined ? MALFORMED : { code, message, fields };
  }
  if (isHubCode(code) && code !== 'VALIDATION') return { code, message };
  return { code: 'INTERNAL', message: message === '' ? UNCODED_MESSAGE : message };
}

/** The first error of a GraphQL `errors` list; `malformed` when the list is not one. */
export function parseErrors(value: unknown): HubError {
  if (!Array.isArray(value) || value.length === 0) return MALFORMED;
  const first: unknown = value[0];
  return parseError(first);
}

/**
 * A GraphQL response body (already parsed from JSON) as a result. Errors win over partial data:
 * a caller never sees half an answer as a value.
 */
export function parseResponse(body: unknown): HubResult<JsonObject> {
  if (!isObject(body)) return { ok: false, error: MALFORMED };
  if (body.errors !== undefined) return { ok: false, error: parseErrors(body.errors) };
  return isObject(body.data) ? { ok: true, value: body.data } : { ok: false, error: MALFORMED };
}

/** Whether `body` has the shape of a GraphQL response, so that an HTTP answer counts as contact (NET-004). */
export function isGraphQLResponse(body: unknown): boolean {
  return isObject(body) && (Array.isArray(body.errors) || isObject(body.data));
}
