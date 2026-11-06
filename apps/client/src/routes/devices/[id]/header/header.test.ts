// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { endTransitionsAtOnce } from '$lib/devices/transitions.fixture';
import { device } from '../page.fixture.svelte';
import Header from './Header.svelte';
import { headerLine, type HeaderDevice } from './header';

endTransitionsAtOnce();

let app: ReturnType<typeof mount> | undefined;

afterEach(async () => {
  if (app) await unmount(app);
  app = undefined;
  document.body.replaceChildren();
  vi.unstubAllGlobals();
});

function show(shown: HeaderDevice) {
  const calls = { onConnect: vi.fn<(button: HTMLElement) => void>(), onSettings: vi.fn<(section: string | null) => void>() };
  app = mount(Header, { target: document.body, props: { device: shown, ...calls } });
  flushSync();
  return calls;
}

function labelled(name: string): HTMLButtonElement {
  const found = [...document.querySelectorAll('button')].find(
    (button) => button.getAttribute('aria-label') === name || button.textContent.replace(/\p{Co}/gu, '').trim() === name,
  );
  if (!found) throw new Error(`no button ${name}`);
  return found;
}

describe('headerLine (DEV-020, D-8)', () => {
  it('DEV-020: host, IP, kind, OS and each tag, in that order, with one tooltip for all', () => {
    const line = headerLine(device());
    expect(line.items).toEqual([
      { part: 'host', text: 'nas.home.arpa' },
      { part: 'ip', text: '192.168.1.20' },
      { part: 'kind', text: 'server' },
      { part: 'os', text: 'Linux' },
      { part: 'tag', text: 'lab' },
      { part: 'tag', text: 'storage' },
    ]);
    expect(line.title).toBe('nas.home.arpa · 192.168.1.20 · server · Linux · lab · storage');
  });

  it.each([null, '', '192.168.1.20'])('D-8: the IP is left out when it is %j and the host is 192.168.1.20', (ip) => {
    const line = headerLine(device({ host: '192.168.1.20', ip }));
    expect(line.items.map((item) => item.part)).not.toContain('ip');
  });

  it.each([
    [{ enabled: true, hostKey: device().ssh.hostKey }, 'connect'],
    [{ enabled: true, hostKey: null }, 'test'],
    [{ enabled: false, hostKey: null }, 'set-up'],
  ] as const)('D-8: SSH %j does %s', (ssh, action) => {
    expect(headerLine(device({}, ssh)).ssh).toBe(action);
  });
});

describe('Header (DEV-020–022)', () => {
  it('DEV-020: one line from the back link to the tags, with the SSH, RDP and Edit buttons last', () => {
    show(device());
    const line = document.querySelector('header');
    const order = [...(line?.querySelectorAll('a, h1, .status, .item, button') ?? [])].map((node) =>
      node instanceof HTMLButtonElement ? `button:${node.textContent.replace(/\p{Co}/gu, '').trim()}` : node.textContent.trim(),
    );
    expect(order).toEqual(['Devices', 'nas', 'Up', 'nas.home.arpa', '192.168.1.20', 'server', 'Linux', 'lab', 'storage', 'button:SSH', 'button:RDP', 'button:Edit']);
  });

  it('DEV-021: every item has its full text as a tooltip, and the group has all of it', () => {
    show(device({ host: 'a-very-long-host-name.example.internal.home.arpa' }));
    const items = [...document.querySelectorAll<HTMLElement>('.details .item')];
    for (const item of items) expect(item.title).toBe(item.textContent);
    expect(document.querySelector<HTMLElement>('.details')?.title).toBe(headerLine(device({ host: 'a-very-long-host-name.example.internal.home.arpa' })).title);
    expect(document.querySelector('h1')?.title).toBe('nas');
  });

});

describe('Header overflow and buttons (DEV-021, DEV-022)', () => {
  it('DEV-021: only the host-to-tags group fades, and only while it is cut', () => {
    const observed: (() => void)[] = [];
    vi.stubGlobal(
      'ResizeObserver',
      class {
        constructor(callback: () => void) {
          observed.push(callback);
        }
        observe() {
          return undefined;
        }
        disconnect() {
          return undefined;
        }
      },
    );
    let scroll = 100;
    vi.spyOn(HTMLElement.prototype, 'scrollWidth', 'get').mockImplementation(() => scroll);
    vi.spyOn(HTMLElement.prototype, 'clientWidth', 'get').mockImplementation(() => 100);
    show(device());
    const group = document.querySelector('.details');
    expect(group?.hasAttribute('data-overflowing')).toBe(false);
    scroll = 300;
    for (const callback of observed) callback();
    expect(group?.hasAttribute('data-overflowing')).toBe(true);
    expect(document.querySelectorAll('[data-overflowing]')).toHaveLength(1);
    // The name, the pill and the buttons are outside the group, so the fade never covers them.
    for (const kept of document.querySelectorAll('h1, .status, .actions')) expect(group?.contains(kept)).toBe(false);
    vi.restoreAllMocks();
  });

});

describe('Header buttons (D-8, DEV-022)', () => {
  it('DEV-022: with SSH off, SSH is dashed with a "+" and opens Settings at SSH, never a session', () => {
    const { onConnect, onSettings } = show(device({}, { enabled: false, hostKey: null }));
    const ssh = labelled('Set up SSH');
    expect(ssh.classList.contains('dashed')).toBe(true);
    expect(ssh.querySelector('[aria-hidden="true"]')).not.toBeNull();
    ssh.click();
    expect(onSettings).toHaveBeenCalledWith('ssh');
    expect(onConnect).not.toHaveBeenCalled();
  });

  it('DEV-022: RDP is always dashed with a "+" and opens Settings at RDP', () => {
    const { onConnect, onSettings } = show(device());
    const rdp = labelled('Set up RDP');
    expect(rdp.classList.contains('dashed')).toBe(true);
    expect(rdp.querySelector('[aria-hidden="true"]')).not.toBeNull();
    rdp.click();
    expect(onSettings).toHaveBeenCalledWith('rdp');
    expect(onConnect).not.toHaveBeenCalled();
  });

  it('D-8: SSH is solid when on, and with a host key it passes its button to start the session', () => {
    const { onConnect, onSettings } = show(device());
    const ssh = labelled('SSH to nas');
    expect(ssh.classList.contains('solid')).toBe(true);
    ssh.click();
    expect(onConnect).toHaveBeenCalledWith(ssh);
    expect(onSettings).not.toHaveBeenCalled();
  });

  it('D-13: while the device loads the line shows only the back link', () => {
    app = mount(Header, { target: document.body, props: { device: null, onConnect: () => undefined, onSettings: () => undefined } });
    flushSync();
    expect(document.querySelector('header')?.textContent.trim()).toBe('Devices');
  });
});
