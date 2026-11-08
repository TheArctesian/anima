import type { FieldError } from '$lib/hub/hub';
import { isSecretHeader } from './config-input';
import { byteLength, integerError, isIntegerIn, LIMITS, lengthError, timeoutError } from './range';
import type { CheckFormValues } from './values';

type HttpValues = CheckFormValues['http'];

const MAX_REDIRECTS = 10;
/** RFC 9110 token: the grammar of a header name. */
const HEADER_TOKEN = /^[!#$%&'*+\-.^_`|~0-9A-Za-z]+$/;
/** Headers that pick the destination; the hub refuses them (`DESTINATION_HEADERS` in config/rules.rs). */
const DESTINATION_HEADERS = ['host', 'forwarded', 'x-forwarded-host', 'x-forwarded-server', 'x-host'];

function urlErrors(text: string): FieldError[] {
  const trimmed = text.trim();
  if (trimmed === '') return [{ field: 'url', message: 'Enter a URL.' }];
  const parsed = URL.canParse(trimmed) ? new URL(trimmed) : null;
  if (parsed === null || (parsed.protocol !== 'http:' && parsed.protocol !== 'https:') || parsed.hostname === '') {
    return [{ field: 'url', message: 'Enter a URL that starts with http:// or https://.' }];
  }
  if (parsed.username !== '' || parsed.password !== '') {
    return [{ field: 'url', message: 'Leave the user name and password out of the URL; use the authentication setting.' }];
  }
  return lengthError('url', trimmed, LIMITS.urlBytes, 'The URL');
}

function headerNameError(name: string): string | undefined {
  if (!HEADER_TOKEN.test(name)) return 'Enter a header name: letters, digits and !#$%&\'*+-.^_`|~ only.';
  if (byteLength(name) > LIMITS.headerNameBytes) return `A header name can be at most ${String(LIMITS.headerNameBytes)} bytes long.`;
  if (DESTINATION_HEADERS.includes(name.toLowerCase())) return 'The URL sets the destination; this header is not allowed.';
  return undefined;
}

function headerErrors(headers: HttpValues['headers']): FieldError[] {
  const rows = headers.map((row, index) => ({ row, index })).filter(({ row }) => row.name.trim() !== '' || row.value !== '');
  const errors: FieldError[] =
    rows.length > LIMITS.headers ? [{ field: 'headers', message: `Use at most ${String(LIMITS.headers)} headers.` }] : [];
  for (const { row, index } of rows) {
    const name = row.name.trim();
    const field = `headers.${String(index)}`;
    const nameError = headerNameError(name);
    if (nameError !== undefined) errors.push({ field: `${field}.name`, message: nameError });
    else if (/[\r\n\0]/.test(row.value)) errors.push({ field: `${field}.value`, message: 'A header value cannot contain line breaks.' });
    else if (row.value === '' && !(row.stored && isSecretHeader(name))) {
      errors.push({ field: `${field}.value`, message: 'Enter the value of this header.' });
    }
  }
  return errors;
}

function rangeErrors(ranges: HttpValues['acceptedStatus']): FieldError[] {
  if (ranges.length === 0) return [{ field: 'acceptedStatus', message: 'Accept at least one status range.' }];
  const errors: FieldError[] =
    ranges.length > LIMITS.statusRanges ? [{ field: 'acceptedStatus', message: `Use at most ${String(LIMITS.statusRanges)} ranges.` }] : [];
  ranges.forEach((range, index) => {
    const field = `acceptedStatus.${String(index)}`;
    if (!isIntegerIn(range.from, 100, 599) || !isIntegerIn(range.to, 100, 599)) {
      errors.push({ field, message: 'Enter statuses from 100 to 599.' });
    } else if (Number(range.from) > Number(range.to)) {
      errors.push({ field, message: 'The first status must not be above the last.' });
    }
  });
  return errors;
}

/** A stored secret may stay empty ("keep"); a new one must be entered. */
function authErrors(http: HttpValues): FieldError[] {
  const errors: FieldError[] = [];
  if (http.authKind === 'BASIC') {
    const username = http.username.trim();
    if (username === '') {
      errors.push({ field: 'username', message: 'Enter the user name. The hub keeps the stored password only for the same user name.' });
    } else if (username.includes(':') || byteLength(username) > LIMITS.usernameBytes) {
      errors.push({ field: 'username', message: `A user name has no ":" and is at most ${String(LIMITS.usernameBytes)} bytes long.` });
    }
    if (http.password === '' && http.storedAuth !== 'BASIC') errors.push({ field: 'password', message: 'Enter the password.' });
  }
  if (http.authKind === 'BEARER' && http.bearerToken === '' && http.storedAuth !== 'BEARER') {
    errors.push({ field: 'bearerToken', message: 'Enter the token.' });
  }
  return errors;
}

function bodyErrors(http: HttpValues): FieldError[] {
  const query = http.jsonQuery.trim();
  const expected = http.jsonExpected.trim();
  return [
    ...lengthError('body', http.body, LIMITS.bodyBytes, 'The request body'),
    ...lengthError('keyword', http.keyword.trim(), LIMITS.keywordBytes, 'The keyword'),
    ...lengthError('jsonQuery', query, LIMITS.jsonQueryBytes, 'The JSON pointer'),
    ...(query !== '' && !query.startsWith('/') ? [{ field: 'jsonQuery', message: 'A JSON pointer starts with "/".' }] : []),
    ...lengthError('jsonExpected', expected, LIMITS.keywordBytes, 'The expected value'),
    ...(expected !== '' && query === '' ? [{ field: 'jsonExpected', message: 'Enter the JSON pointer to compare.' }] : []),
  ];
}

/**
 * Section 6.4 (HTTP): every reason the settings cannot be saved. The authentication is checked only
 * when the config is sent (`configSent`); an edit that leaves it out keeps all stored secrets.
 */
export function validateHttp(http: HttpValues, configSent: boolean): FieldError[] {
  return [
    ...urlErrors(http.url),
    ...timeoutError(http.timeoutMs),
    ...integerError('maxRedirects', http.maxRedirects, [0, MAX_REDIRECTS], `Enter 0 to ${String(MAX_REDIRECTS)} redirects.`),
    ...headerErrors(http.headers),
    ...rangeErrors(http.acceptedStatus),
    ...bodyErrors(http),
    ...(configSent ? authErrors(http) : []),
  ];
}
