// @vitest-environment jsdom
import { flushSync, mount, tick, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { fakeHub } from '$lib/devices/hub.fixture';
import { endTransitionsAtOnce } from '$lib/devices/transitions.fixture';
import Page from './+page.svelte';
import { parseDeviceId } from './device-id';
import { ID, OTHER_ID, answers, beatsPage, check, device, event, heartbeat, route, session } from './page.fixture.svelte';

vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('$lib/motion/engine.fixture')).fakeRun,
}));

const current = vi.hoisted(() => ({ hub: undefined as unknown, goto: [] as [string, unknown][], opened: [] as unknown[] }));
vi.mock('$lib/hub/context', () => ({ useHub: () => current.hub }));
vi.mock('$lib/sessions/sessions.svelte', () => ({
  useSessions: () => ({ openSession: (request: unknown) => current.opened.push(request), openWeb: (request: unknown) => current.opened.push(request) }),
}));
const browser = vi.hoisted(() => vi.fn(() => Promise.resolve()));
// The desktop shell's web view, as `webViewShell` gives it where the platform has web view tabs (WP-42 D-4).
vi.mock('$lib/web-interfaces/shell/client-shell', () => ({ webViewShell: () => ({ openSystemBrowser: browser }) }));
// ...and so these paths run with the desktop's answers: a browser opens a tab of its own instead (WEB-011).
vi.mock('$lib/platform/platform', async () => {
  const { DESKTOP_MAC } = await import('$lib/platform/answers.fixture');
  return { capabilities: () => DESKTOP_MAC };
});
vi.mock('$app/navigation', () => ({
  goto: (href: string, options: unknown) => {
    current.goto.push([href, options]);
    return Promise.resolve();
  },
}));
vi.mock('$app/state', async () => {
  const { route: shown } = await import('./page.fixture.svelte');
  return {
    page: {
      get url() {
        return new URL(`/devices/${shown.param}${shown.search}`, 'http://localhost');
      },
      get params() {
        return { id: shown.param };
      },
      get state() {
        return shown.state;
      },
    },
  };
});

endTransitionsAtOnce();

let app: ReturnType<typeof mount> | undefined;

async function settle(): Promise<void> {
  for (let turn = 0; turn < 8; turn += 1) await tick();
  flushSync();
}

async function show(hubAnswers = answers(), search = ''): Promise<ReturnType<typeof fakeHub>> {
  route.search = search;
  const fake = fakeHub(hubAnswers);
  current.hub = fake.hub;
  app = mount(Page, { target: document.body, props: { data: { id: parseDeviceId(route.param) } } });
  await settle();
  return fake;
}

/** The button with this text (icon glyphs left out) or this accessible label, inside `within` when given. */
function button(label: string, within = 'body'): HTMLButtonElement {
  const found = [...document.querySelectorAll<HTMLButtonElement>(`${within} button`)].find(
    (candidate) => candidate.textContent.replace(/\p{Co}/gu, '').trim() === label || candidate.getAttribute('aria-label') === label,
  );
  if (!found) throw new Error(`no button ${label}`);
  return found;
}

function panel(): HTMLElement | null {
  return document.querySelector('[role="tabpanel"]');
}

beforeEach(() => {
  current.goto = [];
  current.opened = [];
  route.param = ID;
  route.state = {};
});

afterEach(async () => {
  if (app) await unmount(app);
  app = undefined;
  document.body.replaceChildren();
});

describe('route edge (D-14)', () => {
  it('D-14: a malformed ID shows "Device not found" and sends no request', async () => {
    route.param = 'new';
    const fake = await show();
    expect(document.querySelector('h1')?.textContent).toBe('Device not found');
    expect(fake.requests).toEqual([]);
    expect(fake.subscriptions).toEqual([]);
  });

  it('D-14: an ID the hub does not know shows "Device not found"', async () => {
    await show(answers({ Device: { device: null } }));
    expect(document.querySelector('h1')?.textContent).toBe('Device not found');
    expect(document.querySelector('[role="tablist"]')).toBeNull();
  });

  it('K7: a failed load shows the hub message and Retry loads again', async () => {
    const fake = await show(answers({ Device: { code: 'UNAVAILABLE', message: 'The database is unreachable.' } }));
    expect(document.querySelector('[role="alert"]')?.textContent).toContain('The database is unreachable.');
    button('Retry').click();
    await settle();
    expect(fake.requests.filter((request) => request.operation === 'Device')).toHaveLength(2);
  });

  it('T4 M1: the credential error from Add device shows on the new page', async () => {
    route.state = { credentialError: 'The key store is locked.' };
    await show();
    expect(document.querySelector('[role="alert"]')?.textContent).toContain('The key store is locked.');
  });
});

describe('6.7 tabs (D-9, D-10)', () => {
  it.each([
    ['', 'device-panel-overview', '24 hours'],
    ['?tab=checks', 'device-panel-checks', 'ping'],
    ['?tab=sessions', 'device-panel-sessions', 'ada'],
    ['?tab=logs', 'device-panel-logs', 'Check failed'],
    ['?tab=settings', 'device-panel-settings', 'Delete device'],
    ['?tab=unknown', 'device-panel-overview', '24 hours'],
  ])('6.7: the URL %j shows its panel, so a reload keeps the tab', async (search, id, text) => {
    await show(
      answers({
        DeviceSessions: { sessions: { pageInfo: { hasNextPage: false }, edges: [{ node: session('s1', '2026-11-04T10:00:00Z') }] } },
        Events: { events: { pageInfo: { hasNextPage: false }, edges: [{ node: event('e1', ID, 'Check failed') }] } },
      }),
      search,
    );
    expect(panel()?.id).toBe(id);
    expect(panel()?.textContent).toContain(text);
    expect(document.querySelector('[aria-selected="true"]')?.id).toBe(id.replace('panel', 'tab'));
  });

  it('D-9: choosing a tab writes it to the URL, and Back to the old URL shows the old tab', async () => {
    await show();
    button('Checks').click();
    expect(current.goto).toEqual([[`/devices/${ID}?tab=checks`, { keepFocus: true, noScroll: true }]]);
    route.search = '?tab=checks';
    await settle();
    expect(panel()?.id).toBe('device-panel-checks');
    route.search = '';
    await settle();
    expect(panel()?.id).toBe('device-panel-overview');
  });

  it('D-9: a panel loads its data the first time it shows, and only then', async () => {
    const fake = await show(answers({ DeviceSessions: { sessions: { pageInfo: { hasNextPage: false }, edges: [] } } }));
    expect(fake.requests.map((request) => request.operation).filter((operation) => operation !== 'Vpn')).toEqual(['Device', 'DeviceChecks']);
    route.search = '?tab=sessions';
    await settle();
    route.search = '';
    await settle();
    route.search = '?tab=sessions';
    await settle();
    expect(fake.requests.filter((request) => request.operation === 'DeviceSessions')).toEqual([
      { operation: 'DeviceSessions', variables: { deviceId: ID, first: 50, after: null } },
    ]);
  });

  it('D-10: Logs asks for this device\'s events and shows only its live ones', async () => {
    const fake = await show(answers({ Events: { events: { pageInfo: { hasNextPage: false }, edges: [] } } }), '?tab=logs');
    expect(fake.requests.find((request) => request.operation === 'Events')?.variables).toEqual({ filter: { deviceId: ID }, first: 100 });
    fake.push('EventAdded', { eventAdded: event('e2', OTHER_ID, 'Other device') });
    fake.push('EventAdded', { eventAdded: event('e3', ID, 'This device') });
    await settle();
    expect(panel()?.textContent).toContain('This device');
    expect(panel()?.textContent).not.toContain('Other device');
  });
});

describe('header buttons (D-8, DEV-022)', () => {
  it('D-8: SSH with an accepted host key opens a session that grows from the button', async () => {
    await show();
    button('SSH to nas').click();
    expect(current.opened).toEqual([{ deviceId: ID, deviceName: 'nas', os: 'LINUX', protocol: 'SSH', origin: expect.any(Object) as DOMRect }]);
  });

  it('DEV-022: with SSH off, SSH opens Settings at SSH and starts no session', async () => {
    await show(answers({ Device: { device: device({}, { enabled: false, hostKey: null }) } }));
    button('Set up SSH').click();
    expect(current.goto).toEqual([[`/devices/${ID}?tab=settings&section=ssh`, { keepFocus: true, noScroll: true }]]);
    expect(current.opened).toEqual([]);
  });

  it('WP-14 H-1: with SSH on and no host key, SSH opens the connection test in Settings', async () => {
    await show(answers({ Device: { device: device({}, { hostKey: null }) } }));
    button('Test the SSH connection').click();
    expect(current.goto[0]?.[0]).toBe(`/devices/${ID}?tab=settings&section=test`);
    expect(current.opened).toEqual([]);
  });

  it('DEV-022: RDP opens Settings at RDP and never starts a session; Edit opens Settings', async () => {
    await show();
    button('Set up RDP').click();
    button('Edit').click();
    expect(current.goto.map(([href]) => href)).toEqual([`/devices/${ID}?tab=settings&section=rdp`, `/devices/${ID}?tab=settings`]);
    expect(current.opened).toEqual([]);
  });
});

describe('Web button (WEB-003, WEB-011, MOT-005)', () => {
  type OpenIn = 'ANIMA_TAB' | 'SYSTEM_BROWSER';
  const showWith = (url: string, openIn: OpenIn, status: 'UP' | 'DOWN' = 'UP') => {
    const web = { id: 'w1', deviceId: ID, label: 'Admin', url, openIn, pinnedFingerprintSha256: null, pinnedAt: null };
    return show(answers({ Device: { device: device({ status, webInterfaces: [web] }) } }));
  };

  beforeEach(() => {
    browser.mockClear();
  });

  it('WEB-003 / MOT-005: ANIMA_TAB opens a web tab with the device status, growing from the button', async () => {
    await showWith('http://nas.lan/', 'ANIMA_TAB', 'DOWN');
    button('Open Admin of nas').click();
    const web = expect.objectContaining({ id: 'w1', url: 'http://nas.lan/' }) as unknown;
    expect(current.opened).toEqual([expect.objectContaining({ deviceId: ID, deviceName: 'nas', status: 'DOWN', web, origin: expect.any(Object) as DOMRect })]);
    expect(browser).not.toHaveBeenCalled();
  });

  it('WEB-011: SYSTEM_BROWSER over http opens the system browser and no tab', async () => {
    await showWith('http://nas.lan/', 'SYSTEM_BROWSER');
    button('Open Admin of nas').click();
    expect(browser).toHaveBeenCalledExactlyOnceWith('http://nas.lan/');
    expect(current.opened).toEqual([]);
  });

  it('WEB-008: SYSTEM_BROWSER over https opens a web tab, which checks the pin', async () => {
    await showWith('https://nas.lan/', 'SYSTEM_BROWSER');
    button('Open Admin of nas').click();
    expect(browser).not.toHaveBeenCalled();
    expect(current.opened).toHaveLength(1);
  });
});

describe('live updates (D-11)', () => {
  it('D-11: a status change of this device updates the pill; one of another device does not', async () => {
    const fake = await show();
    fake.push('DeviceStatusChanged', { deviceStatusChanged: { deviceId: OTHER_ID, status: 'DOWN', at: '2026-11-04T10:01:00Z' } });
    await settle();
    expect(document.querySelector('header .status')?.textContent).toContain('Up');
    fake.push('DeviceStatusChanged', { deviceStatusChanged: { deviceId: ID, status: 'DOWN', at: '2026-11-04T10:01:00Z' } });
    await settle();
    expect(document.querySelector('header .status')?.textContent).toContain('Down');
  });

  it('D-11: a heartbeat updates its check row and the open history', async () => {
    const fake = await show(answers({ CheckHeartbeats: { check: { id: 'c1', heartbeats: beatsPage(['2026-11-04T10:00:00Z']) } } }), '?tab=checks');
    button('ping').click();
    await settle();
    fake.push('HeartbeatAdded', { heartbeatAdded: heartbeat('2026-11-04T10:01:00Z', 345) });
    await settle();
    expect(document.querySelector('.rows')?.textContent).toContain('345 ms');
    expect(document.querySelectorAll('.beats li')).toHaveLength(2);
  });

  it('D-11: the subscriptions stop when the page goes, the device log\'s too', async () => {
    const fake = await show(answers({ Events: { events: { pageInfo: { hasNextPage: false }, edges: [] } } }), '?tab=logs');
    expect(fake.listeners('EventAdded')).toBe(1);
    expect(fake.listeners('DeviceStatusChanged')).toBe(1);
    expect(fake.listeners('HeartbeatAdded')).toBe(1);
    if (app) await unmount(app);
    app = undefined;
    expect(fake.listeners('DeviceStatusChanged')).toBe(0);
    expect(fake.listeners('HeartbeatAdded')).toBe(0);
    expect(fake.listeners('EventAdded')).toBe(0);
  });
});

describe('checks tab (CHK-005, Section 6.4)', () => {
  it.each(['', '?tab=checks'])('K7: with %j a failed checks load shows the message, and Retry loads again', async (search) => {
    const fake = await show(answers({ DeviceChecks: { code: 'UNAVAILABLE', message: 'The database is unreachable.' } }), search);
    expect(panel()?.textContent).toContain('The database is unreachable.');
    expect(panel()?.textContent).not.toContain('No checks');
    button('Retry').click();
    await settle();
    expect(fake.requests.filter((request) => request.operation === 'DeviceChecks')).toHaveLength(2);
  });

  it('CHK-005: the pause toggle calls setCheckPaused and the row shows the paused check', async () => {
    const fake = await show(answers({ SetCheckPaused: { setCheckPaused: check({ paused: true }) } }), '?tab=checks');
    button('Pause ping').click();
    await settle();
    expect(fake.requests.at(-1)).toEqual({ operation: 'SetCheckPaused', variables: { id: 'c1', paused: true } });
    expect(button('Resume ping').textContent).toContain('Resume');
  });

  it('D-10: a row opens a page of 100 heartbeats, and Load older asks for the next page', async () => {
    const fake = await show(answers({ CheckHeartbeats: { check: { id: 'c1', heartbeats: beatsPage(['2026-11-04T10:00:00Z'], true) } } }), '?tab=checks');
    button('ping').click();
    await settle();
    button('Load older').click();
    await settle();
    expect(fake.requests.filter((request) => request.operation === 'CheckHeartbeats').map((request) => request.variables)).toEqual([
      { id: 'c1', first: 100, after: null },
      { id: 'c1', first: 100, after: 'cursor-1' },
    ]);
    expect(document.querySelector('svg[role="img"]')).not.toBeNull();
  });

  it('6.4: Add check sends the form through createCheck for this device', async () => {
    const created = check({ id: 'c2', name: 'web' });
    const fake = await show(answers({ CreateCheck: { createCheck: created } }), '?tab=checks');
    button('Add check').click();
    await settle();
    const name = document.querySelector<HTMLInputElement>('[role="dialog"] input');
    if (name === null) throw new Error('no name input');
    name.value = 'web';
    name.dispatchEvent(new Event('input', { bubbles: true }));
    button('Add check', '[role="dialog"]').click();
    await settle();
    const sent = fake.requests.find((request) => request.operation === 'CreateCheck');
    expect(sent?.variables).toMatchObject({ input: { deviceId: ID, name: 'web' } });
    expect(document.querySelector('[role="dialog"]')).toBeNull();
    expect(document.querySelector('.rows')?.textContent).toContain('web');
  });

  it('6.4: deleting a check asks first and removes the row', async () => {
    const fake = await show(answers({ DeleteCheck: { deleteCheck: 'c1' } }), '?tab=checks');
    button('Delete ping').click();
    await settle();
    expect(fake.requests.some((request) => request.operation === 'DeleteCheck')).toBe(false);
    button('Delete ping', '[role="dialog"]').click();
    await settle();
    expect(fake.requests.filter((request) => request.operation === 'DeleteCheck')).toHaveLength(1);
    expect(document.querySelector('.rows')?.textContent).toContain('No checks yet');
  });
});
