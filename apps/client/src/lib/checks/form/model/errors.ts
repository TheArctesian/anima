import type { FieldError } from '$lib/hub/hub';
import type { CheckFormValues } from './values';

/** The settings that sit in the Advanced disclosure. */
export const COMMON_FIELDS: ReadonlySet<string> = new Set([
  'intervalSeconds',
  'retries',
  'retryIntervalSeconds',
  'degradedThresholdMs',
]);

/** The first message for `field`, or undefined. */
export function messageFor(errors: readonly FieldError[], field: string): string | undefined {
  return errors.find((error) => error.field === field)?.message;
}

/** Whether any error belongs to a setting inside the Advanced disclosure. */
export function touchesAdvanced(errors: readonly FieldError[]): boolean {
  return errors.some((error) => COMMON_FIELDS.has(error.field));
}

/**
 * The hub names some inputs differently from this form (D-2): its `name` is also the DNS name to
 * look up (its messages for it start "This value", unlike those for the check name), and its `auth`
 * stands for the user name, password or token of the chosen authentication.
 */
export function placeHubErrors(errors: readonly FieldError[], values: CheckFormValues): FieldError[] {
  return errors.map((error) => {
    if (error.field === 'name' && values.type === 'DNS' && error.message.startsWith('This value')) {
      return { ...error, field: 'dnsName' };
    }
    if (error.field === 'auth' && values.type === 'HTTP') {
      const { authKind } = values.http;
      if (authKind === 'BEARER') return { ...error, field: 'bearerToken' };
      if (authKind === 'BASIC') return { ...error, field: error.message.includes('password') ? 'password' : 'username' };
    }
    return error;
  });
}

const BY_TYPE: Record<CheckFormValues['type'], readonly string[]> = {
  ICMP: ['timeoutMs'],
  TCP: ['port', 'timeoutMs'],
  TLS: ['port', 'serverName', 'timeoutMs'],
  HTTP: ['url', 'timeoutMs', 'maxRedirects', 'acceptedStatus', 'headers', 'body', 'keyword', 'jsonQuery', 'jsonExpected', 'auth'],
  DNS: ['dnsName', 'resolver', 'expected'],
  PUSH: [],
  GROUP: ['childIds'],
  DOCKER: ['container', 'timeoutMs'],
  SYSTEMD: ['unit', 'timeoutMs'],
  SNMP: ['port', 'version', 'oid', 'expected', 'community', 'username', 'authPassword', 'privacyPassword', 'timeoutMs'],
  TAILSCALE_PING: ['target', 'timeoutMs'],
};

/** The field names that have an input on screen for `values` (the Advanced ones are opened when needed). */
export function shownFields(values: CheckFormValues): Set<string> {
  const auth = values.type === 'HTTP' ? { BASIC: ['username', 'password'], BEARER: ['bearerToken'], NONE: [] }[values.http.authKind] : [];
  return new Set(['name', ...COMMON_FIELDS, ...BY_TYPE[values.type], ...auth]);
}

/**
 * The errors that no input shows: the form-level ones and the hub's field names this form has no
 * input for. They are listed at the top of the form so that none is lost.
 */
export function unplaced(errors: readonly FieldError[], shown: ReadonlySet<string>): FieldError[] {
  return errors.filter((error) => !shown.has(error.field) && !/^(headers|acceptedStatus)\.\d+/.test(error.field));
}
