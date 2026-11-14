// @vitest-environment jsdom
import { createRawSnippet, flushSync, mount, tick, unmount } from 'svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { DeviceCheck } from '$lib/checks/api';
import { endTransitionsAtOnce } from '$lib/devices/transitions.fixture';
import { warningLines } from '$lib/devices/actions/actions';
import { check, device } from '../page.fixture.svelte';
import DeviceBar from './DeviceBar.svelte';
import type { HeaderDevice } from './header';

endTransitionsAtOnce();

let app: ReturnType<typeof mount> | undefined;

afterEach(async () => {
  if (app) await unmount(app);
  app = undefined;
  document.body.replaceChildren();
});

function show(shown: HeaderDevice | null, extra: { onOpenWeb?: (...args: unknown[]) => void; checks?: readonly DeviceCheck[] } = {}) {
  const calls = { onConnect: vi.fn<(protocol: string, button: HTMLElement) => void>(), onSettings: vi.fn<(section: string | null) => void>() };
  const tabs = createRawSnippet(() => ({ render: () => '<div role="tablist"></div>' }));
  app = mount(DeviceBar, { target: document.body, props: { device: shown, checks: extra.checks ?? null, ...calls, onOpenWeb: extra.onOpenWeb, tabs } });
  flushSync();
  return calls;
}

function web(id: string, label: string) {
  return { id, deviceId: 'd', label, url: `https://${id}.lan`, openIn: 'ANIMA_TAB' as const, pinnedFingerprintSha256: null, pinnedAt: null };
}

/** Text as read, without the icon glyphs (private-use code points), which are hidden from assistive technology. */
const words = (node: Element | null | undefined) => (node?.textContent ?? '').replace(/\p{Co}/gu, '').replace(/\s+/g, ' ').trim();

function labelled(name: string): HTMLButtonElement {
  const found = [...document.querySelectorAll('button')].find((button) => button.getAttribute('aria-label') === name || words(button) === name);
  if (!found) throw new Error(`no button ${name}`);
  return found;
}

/** Opens the connect button's menu. */
async function more(): Promise<void> {
  labelled('More ways to connect to nas').click();
  await tick();
  flushSync();
}

const menu = () => [...document.querySelectorAll('[role="menuitem"]')].map(words);

describe('DeviceBar (D16, DEV-020, DEV-021)', () => {
  it('DEV-020: Devices / the name, the status and the address, then the connect button, Edit and the tabs', () => {
    show(device());
    const header = document.querySelector('header');
    const order = [...(header?.querySelectorAll('a, h1, .status, .mono, button, [role="tablist"]') ?? [])].map((node) =>
      node instanceof HTMLButtonElement ? `button:${node.getAttribute('aria-label') ?? words(node)}` : words(node) || node.getAttribute('role'),
    );
    expect(order).toEqual(['Devices', 'nas', 'Up', 'nas.home.arpa', '192.168.1.20', 'button:SSH to nas', 'button:More ways to connect to nas', 'button:Edit', 'tablist']);
  });

  it('DEV-021: the name has its full text as a tooltip', () => {
    show(device({ name: 'a-very-long-device-name-for-the-bar' }));
    expect(document.querySelector('h1')?.title).toBe('a-very-long-device-name-for-the-bar');
  });

  it('D-13: while the device loads the bar shows only the back link, and no heading', () => {
    show(null);
    expect(words(document.querySelector('header .top'))).toBe('Devices');
    expect(document.querySelector('h1')).toBeNull();
    expect(document.querySelector('header .placeholder')?.getAttribute('aria-hidden')).toBe('true');
  });

  it('D16: an up device has no reason; a down one says why in a sentence and links to its checks', () => {
    show(device());
    expect(document.querySelector('.reason')).toBeNull();
    document.body.replaceChildren();
    const failing = check({ status: 'DOWN', lastHeartbeat: { at: '2026-11-04T10:00:00Z', status: 'DOWN', responseTimeMs: null, message: 'No reply' } });
    show(device({ status: 'DOWN' }), { checks: [failing] });
    expect(words(document.querySelector('.reason'))).toBe('No reply. Review');
    expect(document.querySelector('.reason a')?.getAttribute('href')).toMatch(/\/devices\/[^/]+\?tab=checks$/);
  });
});

describe('DeviceBar connect button (D-8, D16, DEV-022)', () => {
  it('D-8: with a host key SSH leads, and passes its button to start the session', () => {
    const { onConnect, onSettings } = show(device());
    const ssh = labelled('SSH to nas');
    ssh.click();
    expect(onConnect).toHaveBeenCalledWith('SSH', ssh);
    expect(onSettings).not.toHaveBeenCalled();
  });

  it('DEV-022: RDP off is "Set up RDP" in the menu, with a "+", and opens Settings at RDP', async () => {
    const { onConnect, onSettings } = show(device());
    await more();
    expect(menu()).toEqual(['Set up RDP']);
    const rdp = labelled('Set up RDP');
    expect(rdp.querySelector('[aria-hidden="true"]')).not.toBeNull();
    rdp.click();
    expect(onSettings).toHaveBeenCalledWith('rdp');
    expect(onConnect).not.toHaveBeenCalled();
  });

  it('DEV-022: with SSH off and RDP on, RDP leads and "Set up SSH" opens Settings at SSH, never a session', async () => {
    const { onConnect, onSettings } = show(device({ rdp: { ...device().rdp, enabled: true } }, { enabled: false, hostKey: null }));
    const rdp = labelled('RDP to nas');
    rdp.click();
    expect(onConnect).toHaveBeenCalledWith('RDP', rdp);
    await more();
    labelled('Set up SSH').click();
    expect(onSettings).toHaveBeenCalledWith('ssh');
    expect(onConnect).toHaveBeenCalledOnce();
  });

  it('WP-14 H-1: with no host key and nothing else set up, the main action opens the connection test', () => {
    const { onConnect, onSettings } = show(device({}, { hostKey: null }));
    labelled('Test the SSH connection').click();
    expect(onSettings).toHaveBeenCalledWith('test');
    expect(onConnect).not.toHaveBeenCalled();
  });

  it('D-4: VNC is offered only when on, and connects as VNC', async () => {
    show(device());
    await more();
    expect(document.querySelector('[aria-label="VNC to nas"]')).toBeNull();
    document.body.replaceChildren();
    const { onConnect } = show(device({ vnc: { ...device().vnc, enabled: true } }));
    await more();
    labelled('VNC to nas').click();
    expect(onConnect).toHaveBeenCalledWith('VNC', expect.any(HTMLButtonElement));
  });
});

describe('DeviceBar Web, Edit and badge (D-4, RA-020)', () => {
  it('D-4: Edit is last and opens Settings; Web sits before it only when a route gives onOpenWeb', () => {
    const shown = device({ webInterfaces: [web('w1', 'Admin')] });
    const { onSettings } = show(shown);
    expect([...document.querySelectorAll('.actions > button')].map(words)).toEqual(['Edit']);
    labelled('Edit').click();
    expect(onSettings).toHaveBeenCalledWith(null);
    document.body.replaceChildren();
    const open = vi.fn();
    show(shown, { onOpenWeb: open });
    labelled('Open Admin of nas').click();
    expect(open).toHaveBeenCalledWith(shown.webInterfaces[0], labelled('Open Admin of nas'));
    expect(words([...document.querySelectorAll('.actions button')].at(-1))).toBe('Edit');
  });

  it('RA-020: the bar shows a Warning badge with each warning in its tooltip, and none without warnings', () => {
    show(device());
    expect(document.querySelector('.badge')).toBeNull();
    document.body.replaceChildren();
    show(device({ accessWarnings: ['IGNORED_CERTIFICATE', 'UNENCRYPTED_VNC'] }));
    const badge = document.querySelector<HTMLElement>('.badge');
    expect(words(badge)).toBe('Warning');
    expect(badge?.title.split('\n')).toEqual(warningLines(['IGNORED_CERTIFICATE', 'UNENCRYPTED_VNC']));
  });
});
