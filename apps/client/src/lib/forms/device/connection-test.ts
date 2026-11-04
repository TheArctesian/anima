import type { ConnectionTestInput, ConnectionTestResult, HostKey } from './hub-types';
import { credentialInput, sameSsh, sshInput } from './input';
import { normalizedValues, type DeviceFormValues } from './values';

/** Whether the saved device's host and SSH settings are as saved, so that the test can run against the stored record. */
function unchanged(initial: DeviceFormValues | null, values: DeviceFormValues, deviceId: string | null): initial is DeviceFormValues {
  if (initial === null || deviceId === null) return false;
  const before = normalizedValues(initial);
  const now = normalizedValues(values);
  return before.host === now.host && sameSsh(before.ssh, now.ssh);
}

/**
 * DEV-013, D-5: a saved device whose host and SSH settings are unchanged is tested by ID (plus a newly typed
 * credential). Anything else sends the form's values with no ID, since the hub refuses both together.
 */
export function connectionTestInput(initial: DeviceFormValues | null, values: DeviceFormValues, deviceId: string | null): ConnectionTestInput {
  const credential = credentialInput(values);
  const extra = credential === null ? {} : { credential };
  if (unchanged(initial, values, deviceId) && deviceId !== null) return { deviceId, ...extra };
  const now = normalizedValues(values);
  return { host: now.host, ssh: sshInput(now.ssh, initial === null ? undefined : normalizedValues(initial).ssh.port), ...extra };
}

/** DEV-013: whether the test uses the stored credential, which holds only while host and SSH settings are unchanged. */
export function testsStoredCredential(initial: DeviceFormValues | null, values: DeviceFormValues, deviceId: string | null): boolean {
  return unchanged(initial, values, deviceId) && credentialInput(values) === null;
}

export type TestOutcome =
  | { readonly kind: 'ok' }
  | { readonly kind: 'unreachable'; readonly message: string }
  | { readonly kind: 'authFailed'; readonly message: string }
  | { readonly kind: 'hostKeyNew'; readonly hostKey: HostKey }
  | { readonly kind: 'hostKeyChanged'; readonly hostKey: HostKey }
  | { readonly kind: 'hostKeyChangedUnknown' };

const UNREACHABLE_MESSAGE = 'The device did not answer.';
const AUTH_FAILED_MESSAGE = 'The device refused the credential.';

/** DEV-013, SEC-006, SEC-007: one outcome per result. An unreachable host wins, then a changed host key (never ok, even without the key), then a new key, then authentication. */
export function testOutcome(result: ConnectionTestResult): TestOutcome {
  const message = result.message ?? '';
  if (!result.reachable) return { kind: 'unreachable', message: message === '' ? UNREACHABLE_MESSAGE : message };
  if (result.hostKeyChanged) {
    return result.hostKey == null ? { kind: 'hostKeyChangedUnknown' } : { kind: 'hostKeyChanged', hostKey: result.hostKey };
  }
  if (result.hostKey != null) return { kind: 'hostKeyNew', hostKey: result.hostKey };
  if (!result.authenticated) return { kind: 'authFailed', message: message === '' ? AUTH_FAILED_MESSAGE : message };
  return { kind: 'ok' };
}
