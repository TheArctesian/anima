import type { FieldError } from '$lib/hub/hub';
import { isValidHost, normalizeHost } from '$lib/devices/form/host';
import { configChanged } from './config-input';
import { integerError, LIMITS, lengthError, portError, timeoutError } from './range';
import { probeErrors } from '../probes/validate';
import { validateHttp } from './validate-http';
import type { CheckFormValues } from './values';

const DNS_NAME = /^(?=.{1,253}\.?$)([A-Za-z0-9_]([A-Za-z0-9_-]{0,61}[A-Za-z0-9_])?\.)*[A-Za-z0-9_]([A-Za-z0-9_-]{0,61}[A-Za-z0-9_])?\.?$/;

/** An IPv4 or IPv6 literal, not a host name. */
function isIpLiteral(text: string): boolean {
  const host = normalizeHost(text);
  return isValidHost(host) && (host.includes(':') || /^\d+(\.\d+){3}$/.test(host));
}

function dnsErrors(dns: CheckFormValues['dns']): FieldError[] {
  const errors: FieldError[] = [];
  if (dns.name.trim() === '') errors.push({ field: 'dnsName', message: 'Enter the name to look up.' });
  else if (!DNS_NAME.test(dns.name.trim())) errors.push({ field: 'dnsName', message: 'Enter a valid DNS name.' });
  if (dns.resolver.trim() !== '' && !isIpLiteral(dns.resolver.trim())) {
    errors.push({ field: 'resolver', message: 'Enter the resolver as an IP address, or leave it empty.' });
  }
  return [...errors, ...lengthError('expected', dns.expected.trim(), LIMITS.dnsExpectedBytes, 'The expected answer')];
}

function tlsErrors(tls: CheckFormValues['tls']): FieldError[] {
  const serverName = tls.serverName.trim();
  const nameErrors = serverName !== '' && !isValidHost(serverName) ? [{ field: 'serverName', message: 'Enter a valid host name.' }] : [];
  return [...portError(tls.port), ...nameErrors, ...timeoutError(tls.timeoutMs)];
}

function groupErrors(group: CheckFormValues['group']): FieldError[] {
  return group.childIds.length > LIMITS.groupChildren
    ? [{ field: 'childIds', message: `A group takes at most ${String(LIMITS.groupChildren)} checks.` }]
    : [];
}

function typeErrors(values: CheckFormValues, configSent: boolean): FieldError[] {
  switch (values.type) {
    case 'ICMP':
      return timeoutError(values.icmp.timeoutMs);
    case 'TCP':
      return [...portError(values.tcp.port), ...timeoutError(values.tcp.timeoutMs)];
    case 'TLS':
      return tlsErrors(values.tls);
    case 'HTTP':
      return validateHttp(values.http, configSent);
    case 'DNS':
      return dnsErrors(values.dns);
    case 'GROUP':
      return groupErrors(values.group);
    case 'PUSH':
    case 'DOMAIN_EXPIRY': // DOMAIN_EXPIRY editor: WP-50 (D-29); no fields to check yet.
      return [];
    default:
      return probeErrors(values, values.type, configSent);
  }
}

function nameErrors(name: string): FieldError[] {
  const trimmed = name.trim();
  if (trimmed === '') return [{ field: 'name', message: 'Enter a name.' }];
  return [...trimmed].length > LIMITS.nameChars ? [{ field: 'name', message: `A name is at most ${String(LIMITS.nameChars)} characters.` }] : [];
}

function commonErrors(values: CheckFormValues): FieldError[] {
  const { intervalSeconds, retries, retryIntervalSeconds, degradedThresholdMs } = LIMITS;
  const range = ([min, max]: readonly [number, number], what: string) => `${what} from ${String(min)} to ${String(max)}.`;
  return [
    ...nameErrors(values.name),
    ...integerError('intervalSeconds', values.intervalSeconds, intervalSeconds, range(intervalSeconds, 'Enter an interval in seconds')),
    ...integerError('retries', values.retries, retries, range(retries, 'Enter the retries')),
    ...integerError('retryIntervalSeconds', values.retryIntervalSeconds, retryIntervalSeconds, range(retryIntervalSeconds, 'Enter a retry interval in seconds')),
    ...integerError('degradedThresholdMs', values.degradedThresholdMs, degradedThresholdMs, range(degradedThresholdMs, 'Enter a threshold in ms')),
  ];
}

/**
 * Section 6.4: every reason the form cannot be saved, with stable field paths (the hub's names for
 * the settings, so its `VALIDATION` fields land on the same inputs). `initial` is null in create mode.
 */
export function validateCheck(values: CheckFormValues, initial: CheckFormValues | null): FieldError[] {
  const configSent = initial === null || configChanged(initial, values);
  return [...commonErrors(values), ...typeErrors(values, configSent)];
}
