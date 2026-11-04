import type { ConnectionTestInput, ConnectionTestResult, HostKey } from './hub-types';
import { credentialInput, sshInput } from './input';
import { normalizedValues, type DeviceFormValues } from './values';

/** DEV-013, D-5: a saved device with no new credential is tested by ID; anything else sends the unsaved values. */
export function connectionTestInput(values: DeviceFormValues, deviceId: string | null): ConnectionTestInput {
  const credential = credentialInput(values);
  if (deviceId !== null && credential === null) return { deviceId };
  const now = normalizedValues(values);
  return {
    host: now.host,
    ssh: sshInput(now.ssh),
    ...(credential === null ? {} : { credential }),
  };
}

export type TestOutcome =
  | { readonly kind: 'ok' }
  | { readonly kind: 'unreachable'; readonly message: string }
  | { readonly kind: 'authFailed'; readonly message: string }
  | { readonly kind: 'hostKeyNew'; readonly hostKey: HostKey }
  | { readonly kind: 'hostKeyChanged'; readonly hostKey: HostKey };

const UNREACHABLE_MESSAGE = 'The device did not answer.';
const AUTH_FAILED_MESSAGE = 'The device refused the credential.';

/** DEV-013, SEC-006, SEC-007: one outcome per result. An unreachable host wins, then a host key to confirm, then authentication. */
export function testOutcome(result: ConnectionTestResult): TestOutcome {
  const message = result.message ?? '';
  if (!result.reachable) return { kind: 'unreachable', message: message === '' ? UNREACHABLE_MESSAGE : message };
  if (result.hostKey != null) {
    return { kind: result.hostKeyChanged ? 'hostKeyChanged' : 'hostKeyNew', hostKey: result.hostKey };
  }
  if (!result.authenticated) return { kind: 'authFailed', message: message === '' ? AUTH_FAILED_MESSAGE : message };
  return { kind: 'ok' };
}
