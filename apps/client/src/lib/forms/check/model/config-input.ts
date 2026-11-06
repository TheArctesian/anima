import { normalizeHost } from '$lib/forms/device/host';
import type { PendingCheckConfigInput } from '$lib/forms/hub-pending';
import type { CheckConfigInput, HttpAuthInput, HttpConfigInput } from './hub-types';
import { probeInput } from '../probes/config-input';
import { toNumber } from './range';
import type { CheckFormValues } from './values';

/** The header names whose values the hub keeps secret and never returns (as the hub's `is_secret_header`). */
export function isSecretHeader(name: string): boolean {
  return ['authorization', 'proxy-authorization', 'cookie'].includes(name.trim().toLowerCase());
}

/** Trimmed text, or nothing when it is empty: the hub reads a missing optional field as "not set". */
function optional(text: string): string | undefined {
  const trimmed = text.trim();
  return trimmed === '' ? undefined : trimmed;
}

function present<K extends string>(key: K, text: string): Partial<Record<K, string>> {
  const value = optional(text);
  return value === undefined ? {} : ({ [key]: value } as Record<K, string>);
}


/**
 * HTTP authentication as sent. An empty password or token is sent as is: the hub then keeps the
 * stored one if the address, the kind and the user name are unchanged (SEC-005).
 */
function authInput(http: CheckFormValues['http']): HttpAuthInput | undefined {
  if (http.authKind === 'BASIC') return { basic: { username: http.username.trim(), password: http.password } };
  if (http.authKind === 'BEARER') return { bearerToken: http.bearerToken };
  return undefined;
}

function httpInput(http: CheckFormValues['http']): HttpConfigInput {
  const auth = authInput(http);
  return {
    url: http.url.trim(),
    method: http.method,
    headers: http.headers
      .filter((row) => row.name.trim() !== '' || row.value !== '')
      .map((row) => ({ name: row.name.trim(), value: row.value })),
    ...present('body', http.body),
    maxRedirects: toNumber(http.maxRedirects),
    acceptedStatus: http.acceptedStatus.map((row) => ({ from: toNumber(row.from), to: toNumber(row.to) })),
    ...present('keyword', http.keyword),
    ...present('jsonQuery', http.jsonQuery),
    ...present('jsonExpected', http.jsonExpected),
    ...(auth === undefined ? {} : { auth }),
    verifyTls: http.verifyTls,
    timeoutMs: toNumber(http.timeoutMs),
  };
}

/**
 * The `@oneOf` config of `values.type`; settings of the other types are ignored. The Phase 2 types
 * are not in the generated input yet (D-1), so their config is typed through the cast below.
 */
export function configInput(values: CheckFormValues): CheckConfigInput & PendingCheckConfigInput {
  switch (values.type) {
    case 'ICMP':
      return { icmp: { timeoutMs: toNumber(values.icmp.timeoutMs), ipVersion: values.icmp.ipVersion } };
    case 'TCP':
      return { tcp: { port: toNumber(values.tcp.port), timeoutMs: toNumber(values.tcp.timeoutMs) } };
    case 'TLS':
      return {
        tls: { port: toNumber(values.tls.port), ...present('serverName', values.tls.serverName), timeoutMs: toNumber(values.tls.timeoutMs) },
      };
    case 'HTTP':
      return { http: httpInput(values.http) };
    case 'DNS': {
      const { dns } = values;
      const resolver = optional(dns.resolver);
      return {
        dns: {
          name: dns.name.trim(),
          recordType: dns.recordType,
          ...(resolver === undefined ? {} : { resolver: normalizeHost(resolver) }),
          ...present('expected', dns.expected),
        },
      };
    }
    case 'PUSH':
      return { push: true };
    case 'GROUP':
      return { group: { childIds: [...values.group.childIds].sort() } };
    default:
      return probeInput(values, values.type) as CheckConfigInput & PendingCheckConfigInput;
  }
}

/**
 * Whether the config differs from the initial one. `UpdateCheckInput.config` replaces the whole
 * config, so an unchanged config is left out and every stored secret stays untouched.
 * Both sides are built the same way, so an empty secret on both sides is no change.
 */
export function configChanged(initial: CheckFormValues, values: CheckFormValues): boolean {
  const same = JSON.stringify(configInput(initial)) === JSON.stringify(configInput({ ...values, type: initial.type }));
  return !same || initial.http.authKind !== values.http.authKind;
}
