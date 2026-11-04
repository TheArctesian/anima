import type { FieldError } from '$lib/hub/error';
import { isValidHost, normalizeHost } from '../../device/host';
import { configChanged } from './config-input';
import { integerError, MAX_INT, portError, timeoutError } from './range';
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
  return errors;
}

function typeErrors(values: CheckFormValues, configSent: boolean): FieldError[] {
  switch (values.type) {
    case 'ICMP':
      return timeoutError(values.icmp.timeoutMs);
    case 'TCP':
      return [...portError(values.tcp.port), ...timeoutError(values.tcp.timeoutMs)];
    case 'TLS': {
      const serverName = values.tls.serverName.trim();
      const nameErrors = serverName !== '' && !isValidHost(serverName) ? [{ field: 'serverName', message: 'Enter a valid host name.' }] : [];
      return [...portError(values.tls.port), ...nameErrors, ...timeoutError(values.tls.timeoutMs)];
    }
    case 'HTTP':
      return validateHttp(values.http, configSent);
    case 'DNS':
      return dnsErrors(values.dns);
    case 'PUSH':
    case 'GROUP':
      return [];
  }
}

function commonErrors(values: CheckFormValues): FieldError[] {
  return [
    ...(values.name.trim() === '' ? [{ field: 'name', message: 'Enter a name.' }] : []),
    ...integerError('intervalSeconds', values.intervalSeconds, 1, MAX_INT, 'Enter an interval of 1 second or more.'),
    ...integerError('retries', values.retries, 0, MAX_INT, 'Enter 0 or more retries.'),
    ...integerError('retryIntervalSeconds', values.retryIntervalSeconds, 1, MAX_INT, 'Enter a retry interval of 1 second or more.'),
    ...integerError('degradedThresholdMs', values.degradedThresholdMs, 1, MAX_INT, 'Enter a threshold of 1 ms or more.'),
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
