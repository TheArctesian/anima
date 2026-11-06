// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { CERTIFICATE, fakeHub } from '../../fake/hub.fixture';
import { installBrowserFakes } from '../view.fixture.svelte';
import type { SessionsApi } from '../../api';
import CertificateModal from './CertificateModal.svelte';
import { PROBLEM } from './certificate';

let cleanups: (() => unknown)[] = [];

beforeEach(() => {
  cleanups = [installBrowserFakes()];
});

afterEach(async () => {
  for (const cleanup of cleanups.reverse()) await cleanup();
  document.body.innerHTML = '';
});

/** Let the fake hub's answers arrive, then flush Svelte's updates. */
async function settle() {
  for (let turn = 0; turn < 5; turn += 1) await new Promise((resolve) => setTimeout(resolve, 0));
  flushSync();
}

/** The modal over a fake hub whose device's accepted certificate is an old one; `api` replaces calls. */
async function showModal(api: Partial<SessionsApi> = {}) {
  const fake = fakeHub({ acceptedFingerprint: 'sha256:old' });
  const heard = { closed: 0, accepted: 0 };
  const component = mount(CertificateModal, {
    target: document.body,
    props: {
      device: { id: 'device-1', name: 'desk' },
      api: { ...fake.api, ...api },
      onclose: () => (heard.closed += 1),
      onaccepted: () => (heard.accepted += 1),
    },
  });
  cleanups.push(() => fake.hub.close(), () => unmount(component));
  await settle();
  return { ...fake, heard };
}

const dialog = () => document.querySelector<HTMLElement>('[role="alertdialog"]');
const shown = (field: string) => dialog()?.querySelector(`[data-${field}]`)?.textContent;
const button = (name: string) =>
  [...(dialog()?.querySelectorAll('button') ?? [])].find((candidate) => candidate.textContent.trim() === name);

describe('certificate modal (SEC-007, K4 §3)', () => {
  it('SEC-007: reads the new certificate with an RDP connection test and shows its fingerprint, subject and end', async () => {
    const { named } = await showModal();
    expect(named('TestRemoteConnection').map(({ variables }) => variables)).toEqual([{ input: { deviceId: 'device-1', protocol: 'RDP' } }]);
    expect(shown('fingerprint')).toBe(CERTIFICATE.fingerprintSha256);
    expect(shown('subject')).toBe(CERTIFICATE.subject);
    expect(shown('not-after')).toBe(CERTIFICATE.notAfter);
    expect(dialog()?.getAttribute('aria-modal')).toBe('true');
    expect(document.activeElement).toBe(button('Cancel'));
  });

  it('SEC-007: Accept sends exactly the fingerprint read, once, then asks the caller to reconnect', async () => {
    const { named, heard } = await showModal();
    const accept = button('Accept new certificate');
    accept?.click();
    accept?.click();
    await settle();
    expect(named('AcceptServerCertificate').map(({ variables }) => variables)).toEqual([
      { deviceId: 'device-1', fingerprintSha256: CERTIFICATE.fingerprintSha256 },
    ]);
    expect(heard).toEqual({ closed: 0, accepted: 1 });
  });

  it('SEC-007: a CONFLICT reads the certificate again and shows the new one before any second accept', async () => {
    const { device, named, heard } = await showModal();
    device.certificate = { ...CERTIFICATE, fingerprintSha256: 'sha256:newer', subject: null, notAfter: null };
    button('Accept new certificate')?.click();
    await settle();
    expect(named('TestRemoteConnection')).toHaveLength(2);
    expect(shown('fingerprint')).toBe('sha256:newer');
    expect(shown('subject')).toBeUndefined();
    expect(dialog()?.textContent).toContain(PROBLEM.changed);
    expect(heard.accepted).toBe(0);
    button('Accept new certificate')?.click();
    await settle();
    expect(named('AcceptServerCertificate').map(({ variables }) => variables.fingerprintSha256)).toEqual([CERTIFICATE.fingerprintSha256, 'sha256:newer']);
    expect(heard.accepted).toBe(1);
  });

  it('R-035: a read that fails shows fixed text, never the hub message, and offers no accept', async () => {
    await showModal({ testConnection: () => Promise.resolve({ ok: false, error: { code: 'INTERNAL', message: 'HUBMESSAGE' } }) });
    expect(dialog()?.textContent).toContain(PROBLEM.read);
    expect(dialog()?.textContent).not.toContain('HUBMESSAGE');
    expect(button('Accept new certificate')).toBeUndefined();
    expect(button('Read again')).toBeDefined();
  });

  it('Cancel and Escape close without accepting', async () => {
    const { named, heard } = await showModal();
    button('Cancel')?.click();
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true }));
    expect(heard).toEqual({ closed: 2, accepted: 0 });
    expect(named('AcceptServerCertificate')).toEqual([]);
  });
});
