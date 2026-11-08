// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { BROWSER, DESKTOP_MAC } from '$lib/platform/answers.fixture';
import type { Capabilities } from '$lib/platform/platform';
import type { WebInterface } from '$lib/web-interfaces/api';
import { displayFingerprint, fingerprintFrom } from '$lib/web-interfaces/shell/fingerprint';
import WebMenu from '$lib/devices/actions/WebMenu.svelte';
import { endTransitionsAtOnce } from '$lib/devices/transitions.fixture';
import PinnedFingerprint from './PinnedFingerprint.svelte';

const platform = vi.hoisted(() => ({ answers: undefined as unknown as Capabilities }));
vi.mock('$lib/platform/platform', () => ({ capabilities: () => platform.answers }));
endTransitionsAtOnce();

const PIN = 'ab'.repeat(32);
const SHOWN = displayFingerprint(fingerprintFrom(PIN) as NonNullable<ReturnType<typeof fingerprintFrom>>);
const web = (id: string, url: string, pin: string | null): WebInterface => ({ id, deviceId: 'a', label: id, url, openIn: 'ANIMA_TAB', pinnedFingerprintSha256: pin, pinnedAt: null });

let app: ReturnType<typeof mount> | undefined;
afterEach(async () => {
  if (app) await unmount(app);
  app = undefined;
  document.body.replaceChildren();
});

describe('PinnedFingerprint (WEB-011, D-5)', () => {
  it('WEB-011: shows the pin as the trust page writes it, under its label', () => {
    app = mount(PinnedFingerprint, { target: document.body, props: { id: 'pin', webInterface: web('nas', 'https://nas.example.net/', PIN) } });
    flushSync();
    expect(document.body.textContent).toContain('Pinned certificate SHA-256');
    expect(document.querySelector('code')?.textContent).toBe(SHOWN);
  });

  it.each([
    ['no pin', web('nas', 'https://nas.example.net/', null)],
    ['an http interface', web('nas', 'http://nas.example.net/', PIN)],
    ['an ftp interface', web('nas', 'ftp://nas.example.net/', PIN)],
    ['a malformed pin', web('nas', 'https://nas.example.net/', 'ABC')],
  ])('WEB-011: renders nothing for %s', (_name, webInterface) => {
    app = mount(PinnedFingerprint, { target: document.body, props: { id: 'pin', webInterface } });
    flushSync();
    expect(document.body.textContent).toBe('');
  });
});

describe('WebMenu pin (WEB-011)', () => {
  function show(interfaces: WebInterface[], answers: Capabilities, onOpen = vi.fn()) {
    platform.answers = answers;
    app = mount(WebMenu, { target: document.body, props: { deviceName: 'nas', interfaces, onOpen } });
    flushSync();
  }

  const button = () => document.querySelector<HTMLButtonElement>('.device-action');
  const described = (choice: Element | null) => document.getElementById(choice?.getAttribute('aria-describedby') ?? '')?.querySelector('code')?.textContent;

  it('WEB-011: a browser opens the menu for one pinned interface, and the choice is described by its own pin', () => {
    const onOpen = vi.fn();
    show([web('a', 'https://nas.example.net/', PIN)], BROWSER, onOpen);
    expect(document.querySelector('code')).toBeNull();
    button()?.click();
    flushSync();
    expect(onOpen).not.toHaveBeenCalled();
    expect(described(document.querySelector('.choice'))).toBe(SHOWN);
  });

  it('WEB-011: with two interfaces and one pin, the pin belongs to its own choice and the other has none', () => {
    show([web('a', 'https://a.example.net/', null), web('b', 'https://b.example.net/', PIN)], BROWSER);
    button()?.click();
    flushSync();
    const [first, second] = [...document.querySelectorAll('.row')];
    expect(first?.querySelector('code')).toBeNull();
    expect(first?.querySelector('.choice')?.hasAttribute('aria-describedby')).toBe(false);
    expect(second?.querySelector('.url')?.textContent).toBe('https://b.example.net/');
    expect(described(second?.querySelector('.choice') ?? null)).toBe(SHOWN);
    expect(second?.querySelector('.choice')?.textContent).not.toContain(SHOWN);
  });

  it('WEB-011: in a browser with no pins one interface opens directly on the click', () => {
    const onOpen = vi.fn();
    show([web('a', 'https://nas.example.net/', null)], BROWSER, onOpen);
    button()?.click();
    expect(onOpen).toHaveBeenCalledOnce();
    expect(document.querySelector('.panel')).toBeNull();
  });

  it('WEB-011: the desktop opens one pinned interface directly and shows no pin', () => {
    const onOpen = vi.fn();
    show([web('a', 'https://nas.example.net/', PIN)], DESKTOP_MAC, onOpen);
    button()?.click();
    flushSync();
    expect(onOpen).toHaveBeenCalledOnce();
    expect(document.body.textContent).not.toContain(SHOWN);
  });

  it('WEB-011: the open menu has no pin of a choice outside its own row', () => {
    show([web('a', 'https://a.example.net/', PIN), web('b', 'https://b.example.net/', 'cd'.repeat(32))], BROWSER);
    button()?.click();
    flushSync();
    expect([...document.querySelectorAll('.row')].map((row) => row.querySelector('code')?.textContent)).toEqual([SHOWN, displayFingerprint('cd'.repeat(32) as never)]);
  });
});
