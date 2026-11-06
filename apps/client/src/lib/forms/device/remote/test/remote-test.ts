import type { ConnectionTestInput } from '$lib/hub/generated/graphql';
import type { PendingConnectionTestInput, PendingConnectionTestResult, Protocol, ServerCertificate } from '../../../hub-pending';
import type { ConnectionTestResult } from '../../connection-test';
import { normalizedValues, type DeviceFormValues } from '../../values';
import { rdpCredentialInput, rdpInput, vncCredentialInput, vncInput } from '../input';

/** The protocols that have their own connection test; SSH keeps `connection-test.ts`. */
export type RemoteProtocol = Exclude<Protocol, 'SSH'>;

/** The connection test input with the K1 additions (DEV-013). */
export type RemoteTestInput = ConnectionTestInput & PendingConnectionTestInput;

/** The connection test answer; the certificate fields are missing until the hub declares K1. */
export type RemoteTestResult = ConnectionTestResult & Partial<PendingConnectionTestResult>;

function sameJson(a: unknown, b: unknown): boolean {
  return JSON.stringify(a) === JSON.stringify(b);
}

/** Whether the saved device's host and `protocol` settings are as saved, so that the test can run against the stored record. */
export function testsSavedRemote(
  protocol: RemoteProtocol,
  initial: DeviceFormValues | null,
  values: DeviceFormValues,
  deviceId: string | null,
): initial is DeviceFormValues {
  if (initial === null || deviceId === null) return false;
  const before = normalizedValues(initial);
  const now = normalizedValues(values);
  if (before.host !== now.host) return false;
  return protocol === 'RDP'
    ? sameJson(rdpInput(before.rdp), rdpInput(now.rdp, before.rdp.port))
    : sameJson(vncInput(before.vnc), vncInput(now.vnc, before.vnc.port));
}

/**
 * DEV-013: a saved device whose host and `protocol` settings are unchanged is tested by ID (plus a newly typed
 * password). Anything else sends the form's values with no ID, since the hub refuses both together. The SSH
 * credential is never part of it, nor is the other protocol's password.
 */
export function remoteTestInput(
  protocol: RemoteProtocol,
  initial: DeviceFormValues | null,
  values: DeviceFormValues,
  deviceId: string | null,
): RemoteTestInput {
  const password = protocol === 'RDP' ? rdpCredentialInput(values) : vncCredentialInput(values);
  const extra = password === null ? {} : { password };
  if (testsSavedRemote(protocol, initial, values, deviceId) && deviceId !== null) return { deviceId, protocol, ...extra };
  const now = normalizedValues(values);
  const before = initial === null ? undefined : normalizedValues(initial);
  const settings =
    protocol === 'RDP' ? { rdp: rdpInput(now.rdp, before?.rdp.port) } : { vnc: vncInput(now.vnc, before?.vnc.port) };
  return { host: now.host, protocol, ...settings, ...extra };
}

export type RemoteOutcome =
  | { readonly kind: 'ok' }
  | { readonly kind: 'unreachable'; readonly message: string }
  | { readonly kind: 'authFailed'; readonly message: string }
  | { readonly kind: 'certificateNew'; readonly certificate: ServerCertificate }
  | { readonly kind: 'certificateChanged'; readonly certificate: ServerCertificate | null };

const UNREACHABLE_MESSAGE = 'The device did not answer.';
const AUTH_FAILED_MESSAGE = 'The device refused the credential.';

/**
 * DEV-013, SEC-007: one outcome per result. An unreachable host wins, then a changed RDP certificate (never ok, even
 * without the certificate), then a new one (not the saved accepted one, `acceptedFingerprint`), then authentication. VNC has no certificate.
 */
export function remoteOutcome(protocol: RemoteProtocol, result: RemoteTestResult, acceptedFingerprint: string | null = null): RemoteOutcome {
  const message = result.message ?? '';
  if (!result.reachable) return { kind: 'unreachable', message: message === '' ? UNREACHABLE_MESSAGE : message };
  if (protocol === 'RDP') {
    const certificate = result.serverCertificate ?? null;
    if (result.certificateChanged === true) return { kind: 'certificateChanged', certificate };
    if (certificate !== null && certificate.fingerprintSha256 !== acceptedFingerprint) return { kind: 'certificateNew', certificate };
  }
  if (!result.authenticated) return { kind: 'authFailed', message: message === '' ? AUTH_FAILED_MESSAGE : message };
  return { kind: 'ok' };
}

const CHANGED_CERTIFICATE_WARNING = 'The server certificate changed. Do not trust this device until you check it.';

/** DEV-013, SEC-007: the sentence for a test outcome. */
export function remoteOutcomeText(outcome: RemoteOutcome): string {
  switch (outcome.kind) {
    case 'ok':
      return 'The device answered and accepted the credential.';
    case 'unreachable':
    case 'authFailed':
      return outcome.message;
    case 'certificateNew':
      return 'The device has a new server certificate. Check the fingerprint before you trust it.';
    case 'certificateChanged':
      return CHANGED_CERTIFICATE_WARNING;
  }
}

/** Whether the outcome is one the operator must not ignore. */
export function isRemoteWarning(outcome: RemoteOutcome): boolean {
  return outcome.kind === 'certificateChanged';
}
