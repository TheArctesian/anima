import { describe, expect, it } from 'vitest';
import type { HubResult } from '$lib/hub/hub';
import type { ConnectionTest, ServerCertificate } from '../../api';
import { PROBLEM, READING, transition, type CertificateEvent, type CertificateStep } from './certificate';

const CERT: ServerCertificate = { fingerprintSha256: 'sha256:aa', subject: 'CN=desk', notAfter: '2027-01-01T00:00:00Z', firstSeenAt: '2026-11-04T10:00:00Z' };
const SHOWN = { fingerprintSha256: CERT.fingerprintSha256, subject: CERT.subject, notAfter: CERT.notAfter };
const tested = (serverCertificate: ServerCertificate | null, reachable = true): HubResult<ConnectionTest> => ({
  ok: true,
  value: { reachable, authenticated: true, hostKey: null, hostKeyChanged: false, message: 'hub text', serverCertificate, certificateChanged: true },
});
const acceptedAs = (fingerprintSha256: string): HubResult<ServerCertificate> => ({ ok: true, value: { ...CERT, fingerprintSha256 } });
const refused = (code: 'CONFLICT' | 'INTERNAL'): HubResult<never> => ({ ok: false, error: { code, message: 'hub text' } });

function run(...events: CertificateEvent[]): CertificateStep {
  return events.reduce((step, event) => transition(step, event) ?? step, READING);
}

const READ: CertificateEvent[] = [{ event: 'readDone', result: tested(CERT) }];
const ACCEPTING: CertificateEvent[] = [...READ, { event: 'accept' }];

describe('certificate warning (SEC-007, K4 §3)', () => {
  it('SEC-007: there is no accept before a certificate was read', () => {
    expect(transition(READING, { event: 'accept' })).toBeUndefined();
  });

  it('SEC-007: the accept carries exactly the certificate read, and a second accept does nothing', () => {
    const accepting = run(...ACCEPTING);
    expect(accepting).toEqual({ step: 'accepting', certificate: SHOWN });
    expect(transition(accepting, { event: 'accept' })).toBeUndefined();
  });

  it('SEC-007: the accept succeeds only when the hub now trusts the fingerprint shown', () => {
    expect(run(...ACCEPTING, { event: 'accepted', result: acceptedAs('sha256:aa') }).step).toBe('accepted');
    expect(run(...ACCEPTING, { event: 'accepted', result: acceptedAs('sha256:bb') })).toEqual({ step: 'shown', certificate: SHOWN, notice: PROBLEM.accept });
    expect(run(...ACCEPTING, { event: 'accepted', result: refused('INTERNAL') })).toEqual({ step: 'shown', certificate: SHOWN, notice: PROBLEM.accept });
  });

  it('SEC-007: a CONFLICT drops the shown certificate and reads again, saying why', () => {
    const again = run(...ACCEPTING, { event: 'accepted', result: refused('CONFLICT') });
    expect(again).toEqual({ step: 'reading', notice: PROBLEM.changed });
    const newer = { ...CERT, fingerprintSha256: 'sha256:bb' };
    expect(run(...ACCEPTING, { event: 'accepted', result: refused('CONFLICT') }, { event: 'readDone', result: tested(newer) })).toEqual({
      step: 'shown',
      certificate: { ...SHOWN, fingerprintSha256: 'sha256:bb' },
      notice: PROBLEM.changed,
    });
  });

  it.each([
    [refused('INTERNAL'), PROBLEM.read],
    [tested(null), PROBLEM.none],
    [tested(null, false), PROBLEM.unreachable],
  ])('a read that gives no certificate fails with fixed text', (result, problem) => {
    expect(run({ event: 'readDone', result })).toEqual({ step: 'failed', problem });
  });

  it('a failed read can be tried again; answers in another step are ignored', () => {
    const failed = run({ event: 'readDone', result: tested(null) });
    expect(transition(failed, { event: 'read' })).toEqual(READING);
    expect(transition(failed, { event: 'readDone', result: tested(CERT) })).toBeUndefined();
    expect(transition(run(...READ), { event: 'accepted', result: acceptedAs('sha256:aa') })).toBeUndefined();
  });
});
