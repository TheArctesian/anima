import type { HubResult } from '$lib/hub/hub';
import type { ConnectionTest, ServerCertificate } from '../../api';

/** The certificate the operator is shown: exactly what the hub's connection test read. */
export interface ShownCertificate {
  readonly fingerprintSha256: string;
  readonly subject: string | null;
  readonly notAfter: string | null;
}

/**
 * Where the certificate warning stands (SEC-007, K4 §3). It opens `reading`; `accepting` sends the shown
 * fingerprint once. `notice` says why a certificate is shown again.
 */
export type CertificateStep =
  | { readonly step: 'reading'; readonly notice?: string }
  | { readonly step: 'failed'; readonly problem: string }
  | { readonly step: 'shown'; readonly certificate: ShownCertificate; readonly notice?: string }
  | { readonly step: 'accepting'; readonly certificate: ShownCertificate }
  | { readonly step: 'accepted'; readonly certificate: ShownCertificate };

export type CertificateEvent =
  | { readonly event: 'read' }
  | { readonly event: 'readDone'; readonly result: HubResult<ConnectionTest> }
  | { readonly event: 'accept' }
  | { readonly event: 'accepted'; readonly result: HubResult<ServerCertificate> };

export const READING: CertificateStep = { step: 'reading' };

/** Fixed texts; the hub's own messages are not shown (R-035, LOG-007). */
export const PROBLEM = {
  read: 'The connection test failed, so the new certificate could not be read. Try again.',
  unreachable: 'The device could not be reached, so the new certificate could not be read. Try again.',
  none: 'The connection test did not read a server certificate. Close this warning and check the device settings.',
  accept: 'The hub did not accept the certificate. Try again, or close this warning.',
  changed: 'The certificate changed again while you were reading it. This is the one the device presents now.',
} as const;

function readDone(notice: string | undefined, result: HubResult<ConnectionTest>): CertificateStep {
  if (!result.ok) return { step: 'failed', problem: PROBLEM.read };
  const { serverCertificate, reachable } = result.value;
  if (serverCertificate === undefined || serverCertificate === null) {
    return { step: 'failed', problem: reachable ? PROBLEM.none : PROBLEM.unreachable };
  }
  const { fingerprintSha256, subject, notAfter } = serverCertificate;
  const certificate = { fingerprintSha256, subject, notAfter };
  return notice === undefined ? { step: 'shown', certificate } : { step: 'shown', certificate, notice };
}

function accepted(certificate: ShownCertificate, result: HubResult<ServerCertificate>): CertificateStep {
  // The hub answers with the certificate it now trusts; anything but the shown fingerprint is not this accept.
  if (result.ok && result.value.fingerprintSha256 === certificate.fingerprintSha256) return { step: 'accepted', certificate };
  // CONFLICT: the device presents another certificate than the one shown, so read it again before any accept.
  if (!result.ok && result.error.code === 'CONFLICT') return { step: 'reading', notice: PROBLEM.changed };
  return { step: 'shown', certificate, notice: PROBLEM.accept };
}

/**
 * The step `event` leads to, or undefined when it does not apply to `current`. Answers apply only in the step
 * that asked for them, so a late answer or a second click does nothing.
 */
export function transition(current: CertificateStep, event: CertificateEvent): CertificateStep | undefined {
  switch (event.event) {
    case 'read':
      return current.step === 'failed' ? READING : undefined;
    case 'readDone':
      return current.step === 'reading' ? readDone(current.notice, event.result) : undefined;
    case 'accept':
      return current.step === 'shown' ? { step: 'accepting', certificate: current.certificate } : undefined;
    case 'accepted':
      return current.step === 'accepting' ? accepted(current.certificate, event.result) : undefined;
  }
}
