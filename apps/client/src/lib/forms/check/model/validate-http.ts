import type { FieldError } from '$lib/hub/error';
import { isSecretHeader } from './config-input';
import { integerError, isIntegerIn, timeoutError } from './range';
import type { CheckFormValues } from './values';

type HttpValues = CheckFormValues['http'];

const MAX_HEADERS = 20;
const MAX_REDIRECTS = 10;
/** RFC 9110 token: the grammar of a header name. */
const HEADER_TOKEN = /^[!#$%&'*+\-.^_`|~0-9A-Za-z]+$/;

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
  return [];
}

function headerErrors(headers: HttpValues['headers']): FieldError[] {
  const rows = headers.map((row, index) => ({ row, index })).filter(({ row }) => row.name.trim() !== '' || row.value !== '');
  const errors: FieldError[] = rows.length > MAX_HEADERS ? [{ field: 'headers', message: `Use at most ${String(MAX_HEADERS)} headers.` }] : [];
  for (const { row, index } of rows) {
    const name = row.name.trim();
    if (!HEADER_TOKEN.test(name)) {
      errors.push({ field: `headers.${String(index)}.name`, message: 'Enter a header name: letters, digits and !#$%&\'*+-.^_`|~ only.' });
    } else if (row.value === '' && !(row.stored && isSecretHeader(name))) {
      errors.push({ field: `headers.${String(index)}.value`, message: 'Enter the value of this header.' });
    }
  }
  return errors;
}

function rangeErrors(ranges: HttpValues['acceptedStatus']): FieldError[] {
  if (ranges.length === 0) return [{ field: 'acceptedStatus', message: 'Accept at least one status range.' }];
  return ranges.flatMap((range, index) => {
    const field = `acceptedStatus.${String(index)}`;
    if (!isIntegerIn(range.from, 100, 599) || !isIntegerIn(range.to, 100, 599)) {
      return [{ field, message: 'Enter statuses from 100 to 599.' }];
    }
    return Number(range.from) > Number(range.to) ? [{ field, message: 'The first status must not be above the last.' }] : [];
  });
}

/** A stored secret may stay empty ("keep"); a new one must be entered. */
function authErrors(http: HttpValues): FieldError[] {
  const errors: FieldError[] = [];
  if (http.authKind === 'BASIC') {
    if (http.username.trim() === '') {
      errors.push({ field: 'username', message: 'Enter the user name. The hub keeps the stored password only for the same user name.' });
    }
    if (http.password === '' && http.storedAuth !== 'BASIC') errors.push({ field: 'password', message: 'Enter the password.' });
  }
  if (http.authKind === 'BEARER' && http.bearerToken === '' && http.storedAuth !== 'BEARER') {
    errors.push({ field: 'bearerToken', message: 'Enter the token.' });
  }
  return errors;
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
    ...(configSent ? authErrors(http) : []),
  ];
}
