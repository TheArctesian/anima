// @vitest-environment jsdom
import { flushSync } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { installAnimations } from '../../../animations.fixture';
import type { ServerCertificate } from '../../../hub-pending';
import { OK, SAVED, button, leave, save, settle, show, type } from '../../form/form.fixture';
import { testResult } from '../../result.fixture';
import type { DeviceFields } from '../../values';
import type { RemoteTestResult } from './remote-test';

vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('../../../../motion/engine.fixture')).fakeRun,
}));

beforeEach(() => {
  installAnimations();
});

afterEach(leave);

const CERTIFICATE: ServerCertificate = { fingerprintSha256: 'AA:BB:CC', subject: 'CN=desk', notAfter: '2027-01-02T00:00:00Z', firstSeenAt: '2026-11-04T09:00:00Z' };
const WITH_RDP: DeviceFields = {
  ...SAVED,
  rdp: {
    enabled: true, port: 3389, username: 'admin', domain: null, security: 'NLA', ignoreCertificate: false,
    resizeMethod: 'DISPLAY_UPDATE', audioOut: true, microphone: false, sharedDrive: false, credentialStored: false,
  },
  vnc: { enabled: true, port: 5900, username: null, credentialStored: false },
};

function answer(extra: Partial<RemoteTestResult>): { ok: true; value: RemoteTestResult } {
  return { ok: true, value: { ...testResult({ reachable: true, authenticated: false, hostKeyChanged: false }), ...extra } };
}

function rdpTest(): HTMLButtonElement {
  const found = document.querySelector<HTMLButtonElement>('[data-connection-test="rdp"]');
  if (!found) throw new Error('no RDP test button');
  return found;
}

it('DEV-013, SEC-007: a new RDP certificate shows its fingerprint, subject and expiry, and Accept calls back', async () => {
  const accept = vi.fn(() => Promise.resolve(OK));
  const calls = show(WITH_RDP, { section: 'rdp', onAcceptServerCertificate: accept }, answer({ serverCertificate: CERTIFICATE, certificateChanged: false }));
  rdpTest().click();
  await settle();

  expect(calls.onTest).toHaveBeenCalledWith({ deviceId: 'd1', protocol: 'RDP' });
  const seen = document.querySelector('[aria-label="Server certificate seen"]');
  expect(seen?.textContent).toContain('AA:BB:CC');
  expect(seen?.textContent).toContain('CN=desk');
  expect(seen?.textContent).toContain('2027-01-02');

  button('Accept certificate').click();
  await settle();
  expect(accept).toHaveBeenCalledWith('AA:BB:CC');
  expect(document.body.textContent).toContain('Certificate accepted');
});

it('D-4: without the callback there is no Accept certificate button', async () => {
  show(WITH_RDP, { section: 'rdp' }, answer({ serverCertificate: CERTIFICATE, certificateChanged: false }));
  rdpTest().click();
  await settle();
  expect(document.body.textContent).toContain('AA:BB:CC');
  expect([...document.querySelectorAll('button')].some((found) => found.textContent.includes('Accept certificate'))).toBe(false);
});

it('SEC-007: a changed RDP certificate is a warning, never the ok sentence', async () => {
  show(WITH_RDP, { section: 'rdp', onAcceptServerCertificate: () => Promise.resolve(OK) }, answer({ authenticated: true, serverCertificate: CERTIFICATE, certificateChanged: true }));
  rdpTest().click();
  await settle();
  expect(document.body.textContent).toContain('The server certificate changed');
  expect(document.body.textContent).not.toContain('accepted the credential');
  expect(document.querySelector('[role="status"] .warning')).not.toBeNull();
});

it('DEV-013: the VNC test is tested by ID and shows an auth failure', async () => {
  const calls = show(WITH_RDP, { section: 'vnc' }, answer({ message: 'The VNC password was refused.' }));
  document.querySelector<HTMLButtonElement>('[data-connection-test="vnc"]')?.click();
  flushSync();
  await settle();
  expect(calls.onTest).toHaveBeenCalledWith({ deviceId: 'd1', protocol: 'VNC' });
  expect(document.body.textContent).toContain('The VNC password was refused.');
});

it('DEV-013: for an unsaved device a new certificate shows the save note and no Accept', async () => {
  show(null, { section: 'rdp', onAcceptServerCertificate: () => Promise.resolve(OK) }, answer({ serverCertificate: CERTIFICATE, certificateChanged: false }));
  document.querySelector<HTMLInputElement>('[data-section="rdp"] input[type="checkbox"]')?.click();
  flushSync();
  type('host', 'desk.home.arpa');
  type('rdp-username', 'admin');
  rdpTest().click();
  await settle();
  expect(document.body.textContent).toContain('Save the device, then test again to accept this certificate.');
  expect([...document.querySelectorAll('button')].some((found) => found.textContent.includes('Accept certificate'))).toBe(false);
});

it('SEC-007: a changed certificate on a saved device warns and Accept (danger) calls back with the fingerprint', async () => {
  const accept = vi.fn(() => Promise.resolve(OK));
  show(WITH_RDP, { section: 'rdp', onAcceptServerCertificate: accept }, answer({ serverCertificate: CERTIFICATE, certificateChanged: true }));
  rdpTest().click();
  await settle();
  expect(document.body.textContent).toContain('The server certificate changed');
  const accepting = button('Accept certificate');
  expect(accepting.classList.contains('danger')).toBe(true);
  accepting.click();
  await settle();
  expect(accept).toHaveBeenCalledWith('AA:BB:CC');
});

it('DEV-013: the saved accepted certificate seen again gives ok and no second fingerprint block', async () => {
  const saved = { ...WITH_RDP, rdp: { ...WITH_RDP.rdp, credentialStored: false, serverCertificate: CERTIFICATE } } as DeviceFields;
  show(saved, { section: 'rdp' }, answer({ authenticated: true, serverCertificate: CERTIFICATE, certificateChanged: false }));
  rdpTest().click();
  await settle();
  expect(document.body.textContent).toContain('accepted the credential');
  expect(document.querySelector('[aria-label="Server certificate seen"]')).toBeNull();
});

it('DEV-013: a save restarts the test, so a shown result does not outlive it', async () => {
  show(WITH_RDP, { section: 'rdp' }, answer({ authenticated: true }));
  rdpTest().click();
  await settle();
  expect(document.body.textContent).toContain('accepted the credential');
  await save();
  expect(document.body.textContent).not.toContain('accepted the credential');
});

it('DEV-011: a save empties the typed RDP password, which a later test then does not send', async () => {
  const calls = show(WITH_RDP, { section: 'rdp' }, answer({ authenticated: true }));
  type('rdp-password', 'rdp-secret');
  rdpTest().click();
  await settle();
  expect(calls.onTest).toHaveBeenLastCalledWith({ deviceId: 'd1', protocol: 'RDP', password: { password: 'rdp-secret' } });
  await save();
  rdpTest().click();
  await settle();
  expect(calls.onTest).toHaveBeenLastCalledWith({ deviceId: 'd1', protocol: 'RDP' });
});
