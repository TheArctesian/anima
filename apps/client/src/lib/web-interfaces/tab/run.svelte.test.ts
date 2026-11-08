// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { displayFingerprint, fingerprintFrom, type Fingerprint } from '$lib/web-interfaces/shell/shell';
import { button, field, installLayout, pageField, settle, showTab as show, type Shown } from './fake/shell.fixture.svelte';

const PIN = fingerprintFrom('ab'.repeat(32)) as Fingerprint;
const NEW = fingerprintFrom('cd'.repeat(32)) as Fingerprint;
const OLD = fingerprintFrom('ef'.repeat(32)) as Fingerprint;

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

/** A promise and the function that resolves it. */
function held<T = void>() {
  let release: (value: T) => void = () => undefined;
  const promise = new Promise<T>((resolve) => (release = resolve));
  return { promise, release };
}

describe('web tab restarts and late answers (H-4, WEB-008)', () => {
  it('H-4: an open a restart superseded never lets the closed view speak to the next run', async () => {
    const { emit, accepts, props } = await showTab();
    const { shell } = props;
    const open = shell.open.bind(shell);
    const close = shell.close.bind(shell);
    const opening = held();
    const closing = held();
    let opens = 0;
    let closes = 0;
    shell.open = async (target) => {
      opens += 1;
      if (opens === 1) await opening.promise;
      return open(target);
    };
    shell.close = async (tabId) => {
      closes += 1;
      if (closes === 2) await closing.promise;
      return close(tabId);
    };
    props.webInterface = { ...props.webInterface, url: 'https://second.lan/' };
    await settle();
    props.webInterface = { ...props.webInterface, url: 'https://third.lan/' };
    await settle();
    opening.release();
    await settle();
    // The second.lan view is still alive until its queued close runs.
    emit({ tabId: 'tab-1', kind: 'UNTRUSTED', fingerprintSha256: OLD });
    await settle();
    closing.release();
    await settle();
    button('Accept certificate')?.click();
    await settle();
    expect(pageField('presented')).toBeUndefined();
    expect(accepts).toEqual([]);
  });

  it('H-4: events wait for the new view after a restart, then are heard again', async () => {
    const { emit, props } = await showTab();
    const { shell } = props;
    const open = shell.open.bind(shell);
    const opening = held();
    shell.open = async (target) => {
      await opening.promise;
      return open(target);
    };
    props.webInterface = { ...props.webInterface, url: 'https://other.lan/' };
    await settle();
    emit({ tabId: 'tab-1', kind: 'UNTRUSTED', fingerprintSha256: OLD });
    await settle();
    expect(pageField('presented')).toBeUndefined();
    opening.release();
    await settle();
    emit({ tabId: 'tab-1', kind: 'UNTRUSTED', fingerprintSha256: NEW });
    await settle();
    expect(field('presented')).toBe(displayFingerprint(NEW));
  });

});

describe('web tab old-run answers (H-4)', () => {
  it('H-4: a certificate read for the old URL never reaches the new run', async () => {
    const { props } = await showTab({ pinning: false });
    // The first run's read is already waiting; hold the next one too, and answer the old one first.
    const reads = [held<Fingerprint>(), held<Fingerprint>()];
    let read = 0;
    props.shell.readCertificate = () => reads[Math.min((read += 1), 2) - 1]?.promise ?? Promise.reject(new Error('extra read'));
    props.webInterface = { ...props.webInterface, url: 'https://second.lan/' };
    await settle();
    props.webInterface = { ...props.webInterface, url: 'https://third.lan/' };
    await settle();
    reads[0]?.release(OLD);
    await settle();
    expect(pageField('presented')).toBeUndefined();
    reads[1]?.release(NEW);
    await settle();
    expect(pageField('presented')).toBe(displayFingerprint(NEW));
  });

  it('H-4: a restart closes a popped-out view of the old origin too', async () => {
    const { names, props } = await showTab({ url: 'http://printer.lan/' });
    button('Pop out')?.click();
    await settle();
    props.webInterface = { ...props.webInterface, url: 'http://other.lan/' };
    await settle();
    expect(names().slice(names().indexOf('popOut'))).toEqual(['popOut', 'close', 'open', 'show']);
  });
});
