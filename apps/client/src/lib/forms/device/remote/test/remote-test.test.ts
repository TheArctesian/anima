import { describe, expect, it } from 'vitest';
import type { ServerCertificate } from '$lib/sessions/api';
import type { RemoteDeviceFields } from '../values';
import { testResult } from '../../result.fixture';
import { formValues, type DeviceFields, type DeviceFormValues } from '../../values';
import { remoteOutcome, remoteOutcomeText, remoteTestInput } from './remote-test';

const CERTIFICATE: ServerCertificate = { fingerprintSha256: 'AA:BB', subject: 'CN=desk', notAfter: '2027-01-02T00:00:00Z', firstSeenAt: '2026-11-04T09:00:00Z' };

const K1: RemoteDeviceFields = {
  rdp: {
    enabled: true, port: 3389, username: 'admin', domain: null, security: 'NLA', ignoreCertificate: false,
    resizeMethod: 'DISPLAY_UPDATE', audioOut: true, microphone: false, sharedDrive: false, credentialStored: true,
  },
  vnc: { enabled: true, port: 5900, username: null, credentialStored: true },
  macAddress: null,
  metricsSource: 'NONE',
  transfer: { upload: true, download: true, copy: true, paste: true },
  accessWarnings: [],
};
const SAVED: DeviceFields = {
  name: 'desk', host: 'desk.home.arpa', kind: 'computer', os: 'WINDOWS', tags: [],
  ssh: { enabled: true, port: 22, username: 'root', auth: 'PASSWORD' }, ...K1,
};

function edited(change: (values: { -readonly [K in keyof DeviceFormValues]: DeviceFormValues[K] }) => void): DeviceFormValues {
  const values = structuredClone(formValues(SAVED)) as { -readonly [K in keyof DeviceFormValues]: DeviceFormValues[K] };
  change(values);
  return values;
}

function typed(rdpPassword: string, vncPassword: string): DeviceFormValues {
  return edited((values) => {
    values.remoteSecrets = { rdpPassword, vncPassword };
    values.credential = { password: 'ssh-secret', privateKey: 'ssh-key', passphrase: 'ssh-phrase' };
  });
}

describe('DEV-013: remoteTestInput', () => {
  const initial = formValues(SAVED);

  it('tests a saved, unchanged device by ID and protocol only', () => {
    expect(remoteTestInput('RDP', initial, formValues(SAVED), 'd1')).toEqual({ deviceId: 'd1', protocol: 'RDP' });
    expect(remoteTestInput('VNC', initial, formValues(SAVED), 'd1')).toEqual({ deviceId: 'd1', protocol: 'VNC' });
  });

  it('sends the typed password for that protocol only, and no SSH credential', () => {
    const values = typed('rdp-secret', 'vnc-secret');
    expect(remoteTestInput('RDP', initial, values, 'd1')).toEqual({ deviceId: 'd1', protocol: 'RDP', password: { password: 'rdp-secret' } });
    expect(remoteTestInput('VNC', initial, values, 'd1')).toEqual({ deviceId: 'd1', protocol: 'VNC', password: { password: 'vnc-secret' } });
    const text = JSON.stringify([remoteTestInput('RDP', initial, values, 'd1'), remoteTestInput('VNC', initial, values, 'd1')]);
    expect(text).not.toMatch(/ssh-secret|ssh-key|ssh-phrase/);
  });

  it('sends the form values with no ID when the host or that protocol changed', () => {
    const host = edited((values) => { values.host = 'other.home.arpa'; });
    const rdp = remoteTestInput('RDP', initial, host, 'd1');
    expect(rdp).toMatchObject({ host: 'other.home.arpa', protocol: 'RDP', rdp: { enabled: true, port: 3389, username: 'admin' } });
    expect(rdp).not.toHaveProperty('deviceId');
    expect(rdp).not.toHaveProperty('vnc');

    const port = edited((values) => { values.vnc = { ...values.vnc, port: '5901' }; });
    const vnc = remoteTestInput('VNC', initial, port, 'd1');
    expect(vnc).toMatchObject({ host: 'desk.home.arpa', protocol: 'VNC', vnc: { enabled: true, port: 5901 } });
    expect(vnc).not.toHaveProperty('deviceId');
    expect(vnc).not.toHaveProperty('rdp');
  });

  it('keeps a saved RDP test by ID when only the other protocol changed', () => {
    const port = edited((values) => { values.vnc = { ...values.vnc, port: '5901' }; });
    expect(remoteTestInput('RDP', initial, port, 'd1')).toEqual({ deviceId: 'd1', protocol: 'RDP' });
  });

  it('sends the values for a device that is not saved, with the password only when typed', () => {
    const none = remoteTestInput('RDP', null, formValues(SAVED), null);
    expect(none).toMatchObject({ host: 'desk.home.arpa', protocol: 'RDP' });
    expect(none).not.toHaveProperty('password');
    expect(remoteTestInput('RDP', null, typed('rdp-secret', ''), null)).toMatchObject({ password: { password: 'rdp-secret' } });
    expect(remoteTestInput('VNC', null, typed('rdp-secret', ''), null)).not.toHaveProperty('password');
  });
});

describe('DEV-013, SEC-007: remoteOutcome', () => {
  const answer = (partial: Partial<Parameters<typeof testResult>[0]> & { serverCertificate?: ServerCertificate | null; certificateChanged?: boolean }) =>
    ({ ...testResult({ reachable: true, authenticated: true, hostKeyChanged: false }), ...partial });

  it('maps unreachable and reachable', () => {
    expect(remoteOutcome('RDP', answer({ reachable: false, authenticated: false }))).toEqual({ kind: 'unreachable', message: 'The device did not answer.' });
    // The hub always answers `authenticated: false` for RDP and VNC; it is not a failed sign-in.
    expect(remoteOutcome('VNC', answer({ authenticated: false, message: 'bad password' }))).toEqual({ kind: 'reachable', message: 'bad password' });
    expect(remoteOutcome('RDP', answer({ authenticated: false }))).toEqual({ kind: 'reachable', message: '' });
    expect(remoteOutcome('RDP', answer({ serverCertificate: null, certificateChanged: false }), 'AA:BB')).toEqual({ kind: 'reachable', message: '' });
    expect(remoteOutcome('VNC', answer({}))).toEqual({ kind: 'reachable', message: '' });
  });

  it('DEV-013: a reachable result shows the hub message when it has one, else the sign-in sentence', () => {
    expect(remoteOutcomeText(remoteOutcome('RDP', answer({ message: 'No TLS on this port.' })))).toBe('No TLS on this port.');
    expect(remoteOutcomeText(remoteOutcome('RDP', answer({ message: null })))).toBe('The device answered. Anima signs in when a session starts.');
    expect(remoteOutcomeText(remoteOutcome('VNC', answer({ message: '' })))).toBe('The device answered. Anima signs in when a session starts.');
  });

  it('reports a new RDP certificate', () => {
    expect(remoteOutcome('RDP', answer({ authenticated: false, serverCertificate: CERTIFICATE, certificateChanged: false }))).toEqual({
      kind: 'certificateNew',
      certificate: CERTIFICATE,
    });
  });

  it('DEV-013: the saved accepted certificate seen again is not new; a null certificate is no change', () => {
    const same = { serverCertificate: CERTIFICATE, certificateChanged: false };
    expect(remoteOutcome('RDP', answer({ ...same, authenticated: true }), 'AA:BB')).toEqual({ kind: 'reachable', message: '' });
    expect(remoteOutcome('RDP', answer({ ...same, authenticated: false }), 'AA:BB')).toEqual({ kind: 'reachable', message: '' });
    expect(remoteOutcome('RDP', answer(same), 'CC:DD')).toMatchObject({ kind: 'certificateNew' });
  });

  it('never calls a changed RDP certificate ok, with or without the certificate', () => {
    const withIt = remoteOutcome('RDP', answer({ serverCertificate: CERTIFICATE, certificateChanged: true }));
    const without = remoteOutcome('RDP', answer({ serverCertificate: null, certificateChanged: true }));
    expect(withIt).toEqual({ kind: 'certificateChanged', certificate: CERTIFICATE });
    expect(without).toEqual({ kind: 'certificateChanged', certificate: null });
    expect(remoteOutcomeText(without)).toContain('certificate changed');
  });

  it('ignores certificate fields for VNC', () => {
    expect(remoteOutcome('VNC', answer({ serverCertificate: CERTIFICATE, certificateChanged: true }))).toEqual({ kind: 'reachable', message: '' });
  });
});
