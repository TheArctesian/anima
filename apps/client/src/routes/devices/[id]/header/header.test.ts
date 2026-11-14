// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { endTransitionsAtOnce } from '$lib/devices/transitions.fixture';
import { warningLines } from '$lib/devices/actions/actions';
import { device } from '../page.fixture.svelte';
import Header from './Header.svelte';
import { headerLine, kindIcon, protocolButton, type HeaderDevice } from './header';

endTransitionsAtOnce();

let app: ReturnType<typeof mount> | undefined;

afterEach(async () => {
  if (app) await unmount(app);
  app = undefined;
  document.body.replaceChildren();
  vi.unstubAllGlobals();
});

function show(shown: HeaderDevice) {
  const calls = { onConnect: vi.fn<(protocol: string, button: HTMLElement) => void>(), onSettings: vi.fn<(section: string | null) => void>() };
  app = mount(Header, { target: document.body, props: { device: shown, ...calls } });
  flushSync();
  return calls;
}

function web(id: string, label: string) {
  return { id, deviceId: 'd', label, url: `https://${id}.lan`, openIn: 'ANIMA_TAB' as const, pinnedFingerprintSha256: null, pinnedAt: null };
}

/** Text as read, without the icon glyphs (private-use code points), which are hidden from assistive technology. */
const words = (node: Element | null | undefined) => (node?.textContent ?? '').replace(/\p{Co}/gu, '').trim();

function labelled(name: string): HTMLButtonElement {
  const found = [...document.querySelectorAll('button')].find(
    (button) => button.getAttribute('aria-label') === name || button.textContent.replace(/\p{Co}/gu, '').trim() === name,
  );
  if (!found) throw new Error(`no button ${name}`);
  return found;
}

describe('headerLine (DEV-020, D-8)', () => {
  it('DEV-020: host, IP, kind, OS and each tag, in that order, the kind\'s icon and the OS logo', () => {
    const line = headerLine(device());
    expect(line.items).toEqual([
      { part: 'host', text: 'nas.home.arpa' },
      { part: 'ip', text: '192.168.1.20' },
      { part: 'kind', text: 'server' },
      { part: 'os', text: 'Linux' },
      { part: 'tag', text: 'lab' },
      { part: 'tag', text: 'storage' },
    ]);
    expect(line.kindIcon).toBe('carbon:bare-metal-server');
    expect(line.osIcon).toBe('simple-icons:linux');
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

describe('kindIcon', () => {
  it.each([
    ['laptop', 'LINUX', 'carbon:laptop'],
    ['Workstation', 'LINUX', 'carbon:virtual-desktop'],
    ['server', 'MACOS', 'carbon:bare-metal-server'],
    ['router', 'OTHER', 'carbon:router-wifi'],
    ['switch', 'OTHER', 'carbon:router-wifi'],
    ['NAS', 'LINUX', 'carbon:data-base'],
    ['phone', 'OTHER', 'carbon:mobile'],
    ['display', 'OTHER', 'carbon:screen'],
    ['printer', 'LINUX', 'carbon:bare-metal-server'],
    ['printer', 'MACOS', 'carbon:virtual-desktop'],
  ] as const)('the kind %j on %s shows %s', (kind, os, icon) => {
    expect(kindIcon(kind, os)).toBe(icon);
  });
});

describe('protocolButton (DEV-022)', () => {
  it.each([
    ['SSH', 'connect', 'SSH to nas', null],
    ['RDP', 'connect', 'RDP to nas', null],
    ['SSH', 'test', 'Test the SSH connection', 'test'],
    ['SSH', 'set-up', 'Set up SSH', 'ssh'],
    ['RDP', 'set-up', 'Set up RDP', 'rdp'],
  ] as const)('DEV-022: %s in mode %s is named %s and opens %s', (protocol, mode, label, section) => {
    expect(protocolButton(protocol, mode, 'nas')).toEqual({ label, section });
  });

  it('D-4: the line gives RDP connect when on, set-up when off, and VNC only when on', () => {
    expect(headerLine(device()).rdp).toBe('set-up');
    expect(headerLine(device()).vnc).toBe(false);
    const on = headerLine(device({ rdp: { ...device().rdp, enabled: true }, vnc: { ...device().vnc, enabled: true } }));
    expect([on.rdp, on.vnc]).toEqual(['connect', true]);
  });
});

describe('Header (DEV-020–022)', () => {
  it('DEV-020: the back link; the name, status and SSH, RDP and Edit buttons; then the host-to-tags details', () => {
    show(device());
    const header = document.querySelector('header');
    const order = [...(header?.querySelectorAll('a, h1, .status, button, .chip') ?? [])].map((node) =>
      node instanceof HTMLButtonElement ? `button:${words(node)}` : words(node),
    );
    expect(order).toEqual(['Devices', 'nas', 'Up', 'button:SSH', 'button:RDP', 'button:Edit', 'nas.home.arpa', '192.168.1.20', 'server', 'Linux', 'lab', 'storage']);
  });

  it('DEV-021: every detail is whole, with its full text as a tooltip, and the name has its tooltip', () => {
    show(device({ host: 'a-very-long-host-name.example.internal.home.arpa' }));
    const chips = [...document.querySelectorAll<HTMLElement>('[aria-label="Details"] li')];
    expect(chips.map((chip) => chip.title)).toEqual(chips.map((chip) => words(chip)));
    expect(chips[0]?.title).toBe('a-very-long-host-name.example.internal.home.arpa');
    expect(document.querySelector('h1')?.title).toBe('nas');
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
    expect(onConnect).toHaveBeenCalledWith('SSH', ssh);
    expect(onSettings).not.toHaveBeenCalled();
  });

  it('RA-030: with RDP on, RDP is solid and connects with its button, never opening Settings', () => {
    const { onConnect, onSettings } = show(device({ rdp: { ...device().rdp, enabled: true } }));
    const rdp = labelled('RDP to nas');
    expect(rdp.classList.contains('dashed')).toBe(false);
    rdp.click();
    expect(onConnect).toHaveBeenCalledWith('RDP', rdp);
    expect(onSettings).not.toHaveBeenCalled();
  });

  it('D-4: VNC shows only when on, and connects as VNC', () => {
    show(device());
    expect(document.querySelector('[aria-label="VNC to nas"]')).toBeNull();
    document.body.replaceChildren();
    const { onConnect } = show(device({ vnc: { ...device().vnc, enabled: true } }));
    const vnc = labelled('VNC to nas');
    vnc.click();
    expect(onConnect).toHaveBeenCalledWith('VNC', vnc);
  });

});

describe('Header Web, Edit and badge (D-4, RA-020)', () => {
  it('D-4: Edit is last, and Web sits before it only when a route gives onOpenWeb', () => {
    const shown = device({ webInterfaces: [web('w1', 'Admin')] });
    show(shown);
    expect([...document.querySelectorAll('.actions button')].map((node) => node.textContent.replace(/\p{Co}/gu, '').trim())).toEqual(['SSH', 'RDP', 'Edit']);
    document.body.replaceChildren();
    const open = vi.fn();
    app = mount(Header, { target: document.body, props: { device: shown, onConnect: () => undefined, onSettings: () => undefined, onOpenWeb: open } });
    flushSync();
    expect([...document.querySelectorAll('.actions button')].map((node) => node.textContent.replace(/\p{Co}/gu, '').trim())).toEqual(['SSH', 'RDP', 'Web', 'Edit']);
    labelled('Open Admin of nas').click();
    expect(open).toHaveBeenCalledWith(shown.webInterfaces?.[0], labelled('Open Admin of nas'));
  });

  it('RA-020: the header shows a Warning badge with each warning in its tooltip, and none without warnings', () => {
    show(device());
    expect(document.querySelector('.badge')).toBeNull();
    document.body.replaceChildren();
    show(device({ accessWarnings: ['IGNORED_CERTIFICATE', 'UNENCRYPTED_VNC'] }));
    const badge = document.querySelector<HTMLElement>('.badge');
    expect(badge?.textContent.replace(/\p{Co}/gu, '').trim()).toBe('Warning');
    expect(badge?.title.split('\n')).toEqual(warningLines(['IGNORED_CERTIFICATE', 'UNENCRYPTED_VNC']));
  });

  it('D-13: while the device loads the line shows only the back link', () => {
    app = mount(Header, { target: document.body, props: { device: null, onConnect: () => undefined, onSettings: () => undefined } });
    flushSync();
    expect(words(document.querySelector('header'))).toBe('Devices');
  });
});
