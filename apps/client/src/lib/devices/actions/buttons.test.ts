// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { CardDevice } from '$lib/devices/api';
import { device } from '../screen/screen.fixture.svelte';
import { endTransitionsAtOnce } from '../transitions.fixture';
import DeviceCard from '../card/DeviceCard.svelte';
import DeviceTable from '../card/DeviceTable.svelte';

endTransitionsAtOnce();

let app: ReturnType<typeof mount> | undefined;

afterEach(async () => {
  if (app) await unmount(app);
  app = undefined;
  document.body.replaceChildren();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

const KEYED = { enabled: true, hostKey: { fingerprintSha256: 'SHA256:abc' } };
const web = (id: string, label: string) => ({
  id,
  deviceId: 'a',
  label,
  url: `https://${id}.lan/admin`,
  openIn: 'ANIMA_TAB' as const,
  pinnedFingerprintSha256: null,
  pinnedAt: null,
});

type Opener = (device: CardDevice, protocol: string, button: HTMLElement) => void;

function showCard(over: Partial<CardDevice>, withWeb = true) {
  const calls = { onConnect: vi.fn<Opener>(), onOpenWeb: vi.fn() };
  app = mount(DeviceCard, {
    target: document.body,
    props: { device: device('a', over), beats: undefined, onConnect: calls.onConnect, ...(withWeb ? { onOpenWeb: calls.onOpenWeb } : {}) },
  });
  flushSync();
  return calls;
}

const named = (label: string) => document.querySelector<HTMLButtonElement>(`[aria-label="${label}"]`);
const texts = () => [...document.querySelectorAll('footer button')].map((node) => node.textContent.replace(/\p{Co}/gu, '').trim());

describe('card buttons (RA-030, WEB-003, D-4)', () => {
  it('RA-030: SSH, RDP and VNC show when on, and each calls onConnect with its protocol and button', () => {
    const { onConnect } = showCard({ ssh: KEYED, rdp: { enabled: true }, vnc: { enabled: true, username: null } });
    expect(texts()).toEqual(['SSH', 'RDP', 'VNC']);
    for (const protocol of ['SSH', 'RDP', 'VNC']) {
      const button = named(`${protocol} to Device a`);
      button?.click();
      expect(onConnect).toHaveBeenLastCalledWith(expect.objectContaining({ id: 'a' }), protocol, button);
    }
  });

  it('RA-030: a device with RDP and VNC off shows no RDP or VNC button', () => {
    showCard({ ssh: KEYED });
    expect(texts()).toEqual(['SSH']);
  });

  it('WEB-003: a table row has the same buttons and passes the protocol', () => {
    const onConnect = vi.fn<Opener>();
    app = mount(DeviceTable, {
      target: document.body,
      props: { devices: [device('a', { rdp: { enabled: true } })], beats: new Map(), onConnect },
    });
    flushSync();
    named('RDP to Device a')?.click();
    expect(onConnect).toHaveBeenCalledWith(expect.objectContaining({ id: 'a' }), 'RDP', named('RDP to Device a'));
  });

  it('WEB-003: the Web button is hidden without onOpenWeb or without interfaces', () => {
    showCard({ webInterfaces: [web('w1', 'Admin')] }, false);
    expect(texts()).toEqual([]);
    document.body.replaceChildren();
    showCard({ webInterfaces: [] });
    expect(texts()).toEqual([]);
  });

});

describe('Web button (WEB-003)', () => {
  it('WEB-003: with one interface the Web button opens it directly, with the button as origin', () => {
    const { onOpenWeb } = showCard({ webInterfaces: [web('w1', 'Admin')] });
    const button = named('Open Admin of Device a');
    button?.click();
    flushSync();
    expect(onOpenWeb).toHaveBeenCalledWith(expect.objectContaining({ id: 'a' }), expect.objectContaining({ id: 'w1' }), button);
    expect(document.querySelector('.panel')).toBeNull();
  });

  it('WEB-003: with two interfaces the Web button opens a menu of labels and URLs; choosing one opens it', () => {
    const { onOpenWeb } = showCard({ webInterfaces: [web('w1', 'Admin'), web('w2', 'Metrics')] });
    const button = named('Web interfaces of Device a');
    expect(button?.getAttribute('aria-expanded')).toBe('false');
    button?.click();
    flushSync();
    expect(button?.getAttribute('aria-expanded')).toBe('true');
    const choices = [...document.querySelectorAll<HTMLButtonElement>('.panel .choice')];
    expect(choices.map((node) => node.textContent.replace(/\s+/g, ' ').trim())).toEqual([
      'Admin https://w1.lan/admin',
      'Metrics https://w2.lan/admin',
    ]);
    choices[1]?.click();
    flushSync();
    expect(onOpenWeb).toHaveBeenCalledWith(expect.objectContaining({ id: 'a' }), expect.objectContaining({ id: 'w2' }), button);
  });

  it('WEB-003: Escape closes the menu and focus returns to the Web button', async () => {
    showCard({ webInterfaces: [web('w1', 'Admin'), web('w2', 'Metrics')] });
    const button = named('Web interfaces of Device a');
    button?.click();
    flushSync();
    document.querySelector<HTMLElement>('.choice')?.focus();
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
    flushSync();
    await vi.waitFor(() => {
      expect(document.querySelector('.panel')).toBeNull();
    });
    expect(document.activeElement).toBe(button);
  });
});

const openMenu = (): HTMLElement => {
  showCard({ webInterfaces: [web('w1', 'Admin'), web('w2', 'Metrics')] });
  const button = named('Web interfaces of Device a');
  if (button === null || button === undefined) throw new Error('no Web button');
  button.click();
  flushSync();
  expect(document.querySelector('.panel')).not.toBeNull();
  return button;
};
const closed = (): Promise<void> =>
  vi.waitFor(() => {
    expect(document.querySelector('.panel')).toBeNull();
  });

describe('Web menu closing (WEB-003)', () => {
  it('WEB-003: a scroll that moves the button closes the open menu, so it never floats away from its button', async () => {
    const button = openMenu();
    vi.spyOn(button, 'getBoundingClientRect').mockReturnValue(new DOMRect(0, 120, 40, 20));
    document.body.dispatchEvent(new Event('scroll'));
    flushSync();
    await closed();
  });

  it('WEB-003: a scroll that leaves the button where it was keeps the menu open', () => {
    openMenu();
    document.body.dispatchEvent(new Event('scroll'));
    flushSync();
    expect(document.querySelector('.panel')).not.toBeNull();
  });

  it('WEB-003: a resize that changes the width closes the open menu', async () => {
    openMenu();
    vi.stubGlobal('innerWidth', window.innerWidth + 200);
    window.dispatchEvent(new Event('resize'));
    flushSync();
    await closed();
  });

  it('WEB-003: a resize that keeps the width keeps the menu open', () => {
    openMenu();
    window.dispatchEvent(new Event('resize'));
    flushSync();
    expect(document.querySelector('.panel')).not.toBeNull();
  });

  it('WEB-003: a scroll inside the menu keeps it open', () => {
    showCard({ webInterfaces: [web('w1', 'Admin'), web('w2', 'Metrics')] });
    named('Web interfaces of Device a')?.click();
    flushSync();
    document.querySelector('.panel')?.dispatchEvent(new Event('scroll'));
    flushSync();
    expect(document.querySelector('.panel')).not.toBeNull();
  });

  it('WEB-003: a press outside closes the menu', async () => {
    showCard({ webInterfaces: [web('w1', 'Admin'), web('w2', 'Metrics')] });
    named('Web interfaces of Device a')?.click();
    flushSync();
    document.querySelector('.panel')?.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true }));
    flushSync();
    expect(document.querySelector('.panel')).not.toBeNull();
    document.body.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true }));
    flushSync();
    await vi.waitFor(() => {
      expect(document.querySelector('.panel')).toBeNull();
    });
  });
});

describe('warning badge (RA-020, D-6)', () => {
  it('RA-020: a card with warnings shows the word Warning and lists each warning in the tooltip', () => {
    showCard({ accessWarnings: ['RDP_SECURITY', 'UNENCRYPTED_VNC'] });
    const badge = document.querySelector<HTMLElement>('.badge');
    expect(badge?.textContent.replace(/\p{Co}/gu, '').trim()).toBe('Warning');
    // Below bp-compact the word is visually hidden but stays in its own element, so the badge keeps its name (RA-020, UI-003).
    expect(badge?.querySelector('.label')?.textContent).toBe('Warning');
    expect(badge?.title.split('\n')).toHaveLength(2);
    expect(badge?.title).toContain('RDP');
    expect(badge?.title).toContain('VNC');
  });

  it('RA-020: a card without warnings shows no badge, and a table row with warnings shows one', () => {
    showCard({});
    expect(document.querySelector('.badge')).toBeNull();
    document.body.replaceChildren();
    app = mount(DeviceTable, {
      target: document.body,
      props: { devices: [device('a', { accessWarnings: ['IGNORED_CERTIFICATE'] })], beats: new Map(), onConnect: vi.fn() },
    });
    flushSync();
    expect(document.querySelector('tbody .badge')?.getAttribute('title')).toContain('certificate');
  });
});
