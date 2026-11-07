// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { fingerprintFrom, type Fingerprint } from '$lib/desktop/webView/webView';
import { button, field, installLayout, settle, showTab as show, type Shown } from './fake/shell.fixture.svelte';

const PIN = fingerprintFrom('ab'.repeat(32)) as Fingerprint;
const NEW = fingerprintFrom('cd'.repeat(32)) as Fingerprint;

let cleanups: (() => unknown)[] = [];
const showTab = (shown: Shown = {}) => show(shown, cleanups);

beforeEach(() => {
  cleanups = [installLayout()];
});

afterEach(async () => {
  for (const cleanup of cleanups.reverse()) await cleanup();
  document.body.innerHTML = '';
});

describe('web tab run (H-4, WEB-008, SEC-007)', () => {
  it('H-4: a new URL closes the view of the old origin and starts again without the old pin', async () => {
    const { calls, props } = await showTab({ pinned: PIN });
    props.webInterface = { ...props.webInterface, url: 'https://other.lan/', pinnedFingerprintSha256: null };
    await settle();
    const opens = calls.filter(({ command }) => command === 'open' || command === 'close');
    expect(opens.map(({ command }) => command)).toEqual(['open', 'close', 'open']);
    expect(opens[2]).toMatchObject({ opening: { url: 'https://other.lan/', pin: null } });
    expect(field('url')).toBe('https://other.lan/');
    expect(field('tls')).not.toContain('Pinned');
  });

  it('H-4: an accept answered after the URL changed pins nothing in the new view', async () => {
    const { emit, calls, props } = await showTab();
    let answer: () => void = () => undefined;
    const held = new Promise<void>((resolve) => (answer = resolve));
    const accept = props.api.acceptWebCertificate;
    props.api = { acceptWebCertificate: async (id, fingerprint) => (await held, accept(id, fingerprint)) };
    emit({ tabId: 'tab-1', kind: 'UNTRUSTED', fingerprintSha256: NEW });
    await settle();
    button('Accept certificate')?.click();
    await settle();
    props.webInterface = { ...props.webInterface, url: 'https://other.lan/' };
    await settle();
    answer();
    await settle();
    expect(calls.map(({ command }) => command)).not.toContain('pin');
  });

  it('H-4: the pin this tab accepted, coming back from the hub, keeps the view', async () => {
    const { emit, calls, props } = await showTab();
    emit({ tabId: 'tab-1', kind: 'UNTRUSTED', fingerprintSha256: NEW });
    await settle();
    button('Accept certificate')?.click();
    await settle();
    props.webInterface = { ...props.webInterface, pinnedFingerprintSha256: NEW };
    await settle();
    expect(calls.filter(({ command }) => command === 'open' || command === 'close')).toHaveLength(1);
  });

  it('H-4: another pin from the hub starts again with it', async () => {
    const { calls, props } = await showTab({ pinned: PIN });
    props.webInterface = { ...props.webInterface, pinnedFingerprintSha256: NEW };
    await settle();
    expect(calls.filter(({ command }) => command === 'open').at(-1)).toMatchObject({ opening: { pin: NEW } });
  });

  it('WEB-008: a stop that arrives while the certificate is read keeps the system browser closed', async () => {
    const { emit, names, answers } = await showTab({ pinned: PIN, certificate: PIN });
    emit({ tabId: 'tab-1', kind: 'LOADED', url: 'https://nas.lan:5001/' });
    let release: () => void = () => undefined;
    answers.gate = new Promise((resolve) => (release = resolve));
    button('Open in system browser')?.click();
    await settle();
    emit({ tabId: 'tab-1', kind: 'MISMATCH', expected: PIN, presented: NEW });
    release();
    await settle();
    expect(names()).toContain('read');
    expect(names()).not.toContain('browser');
    expect(document.body.textContent).toContain('has changed');
  });
});
