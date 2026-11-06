import { configChanged, isSecretHeader } from './config-input';
import type { CheckFormValues } from './values';

type HttpValues = CheckFormValues['http'];

function originOf(url: string): string | null {
  const text = url.trim();
  return URL.canParse(text) ? new URL(text).origin : null;
}

function storedHeaderNames(http: HttpValues): string[] {
  return http.headers.filter((row) => row.stored && isSecretHeader(row.name)).map((row) => row.name.trim().toLowerCase());
}

/** Every secret-named header that will be sent, stored or new: the hub compares all of them with what it holds. */
function sentSecretHeaderNames(http: HttpValues): string[] {
  return http.headers
    .filter((row) => isSecretHeader(row.name) && (row.stored || row.value !== ''))
    .map((row) => row.name.trim().toLowerCase());
}

/** Whether saving would send an empty secret, which the hub fills in from what it holds. */
function keepsStoredSecret(http: HttpValues): boolean {
  const kept = (http.authKind === 'BASIC' && http.password === '') || (http.authKind === 'BEARER' && http.bearerToken === '');
  const authKept = http.authKind === http.storedAuth && kept;
  return authKept || http.headers.some((row) => row.stored && row.value === '' && isSecretHeader(row.name));
}

/** The hub reuses stored secrets only for the same origin, authentication and secret headers, and with TLS checking kept on. */
function destinationChanged(initial: HttpValues, http: HttpValues): boolean {
  const sameHeaders = JSON.stringify(storedHeaderNames(initial).sort()) === JSON.stringify(sentSecretHeaderNames(http).sort());
  return originOf(initial.url) !== originOf(http.url) || initial.authKind !== http.authKind || (initial.verifyTls && !http.verifyTls) || !sameHeaders;
}

/**
 * Sentences for the operator when saving would lose a stored HTTP secret. `UpdateCheckInput.config`
 * replaces the whole config, and the hub never returns secrets, so the form cannot resend them.
 * It sends them empty to mean "keep"; this lists the cases where that cannot work or removes them.
 */
export function secretWarnings(initial: CheckFormValues, values: CheckFormValues): string[] {
  if (initial.type !== 'HTTP' || !configChanged(initial, values)) return [];
  const { http } = values;
  const warnings: string[] = [];
  if (initial.http.storedAuth !== 'NONE' && http.authKind === 'NONE') {
    warnings.push('Saving removes the stored authentication of this check.');
  }
  const kept = new Set(storedHeaderNames(http));
  for (const name of storedHeaderNames(initial.http)) {
    if (!kept.has(name)) warnings.push(`Saving removes the stored value of the ${name} header.`);
  }
  if (keepsStoredSecret(http) && destinationChanged(initial.http, http)) {
    warnings.push('The address, the authentication or the secret headers changed, so the hub will not reuse the stored secrets. Enter every secret again.');
  }
  return warnings;
}
