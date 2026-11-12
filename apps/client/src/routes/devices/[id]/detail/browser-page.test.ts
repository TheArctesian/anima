// @vitest-environment jsdom
import { flushSync, mount, tick, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { BROWSER, DESKTOP_MAC } from '$lib/platform/answers.fixture';
import { fakeHub } from '$lib/devices/hub.fixture';
import { endTransitionsAtOnce } from '$lib/devices/transitions.fixture';
import Page from '../+page.svelte';
import { parseDeviceId } from '../device-id';
import { ID, OTHER_ID, answers, device, event, route } from '../page.fixture.svelte';

vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('$lib/motion/engine.fixture')).fakeRun,
}));

const current = vi.hoisted(() => ({ hub: undefined as unknown, opened: [] as unknown[], answers: undefined as unknown }));
vi.mock('$lib/hub/context', () => ({ useHub: () => current.hub }));
vi.mock('$lib/sessions/sessions.svelte', () => ({
  useSessions: () => ({ openSession: (request: unknown) => current.opened.push(request), openWeb: (request: unknown) => current.opened.push(request) }),
}));
// The platform answers a test sets: the browser's by default (CLI-001), the desktop's for the positive control.
vi.mock('$lib/platform/platform', () => ({ capabilities: () => current.answers }));
vi.mock('$app/navigation', () => ({ goto: () => Promise.resolve() }));
vi.mock('$app/state', async () => {
  const { route: shown } = await import('../page.fixture.svelte');
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

const panel = () => document.querySelector('[role="tabpanel"]');
const NO_EVENTS = { events: { pageInfo: { hasNextPage: false }, edges: [] } };
/** Every desktop-only control of criterion 2, by the words the operator would read. */
const DESKTOP_ONLY = /updates?\b|log folder|this mac|system settings|pop.?out|web view/i;

beforeEach(() => {
  current.opened = [];
  current.answers = BROWSER;
  route.param = ID;
  route.state = {};
});

afterEach(async () => {
  if (app) await unmount(app);
  app = undefined;
  document.body.replaceChildren();
  vi.restoreAllMocks();
});

describe('the device page with the browser answers (criterion 2, G-1)', () => {
  it('CLI-001, LOG-008: the logs panel renders this device\'s events', async () => {
    await show(answers({ Events: { events: { pageInfo: { hasNextPage: false }, edges: [{ node: event('e1', ID, 'Check failed') }] } } }), '?tab=logs');
    expect(panel()?.id).toBe('device-panel-logs');
    expect(panel()?.textContent).toContain('Check failed');
  });

  it('CLI-001, D-11: the status pill renders, and a live event of this device updates it', async () => {
    const fake = await show(answers({ Events: NO_EVENTS }));
    const pill = () => document.querySelector('header .status')?.textContent;
    expect(pill()).toContain('Up');
    fake.push('DeviceStatusChanged', { deviceStatusChanged: { deviceId: OTHER_ID, status: 'DOWN', at: '2026-11-04T10:01:00Z' } });
    await settle();
    expect(pill()).toContain('Up');
    fake.push('DeviceStatusChanged', { deviceStatusChanged: { deviceId: ID, status: 'DOWN', at: '2026-11-04T10:01:00Z' } });
    await settle();
    expect(pill()).toContain('Down');
  });

  it.each(['', '?tab=checks', '?tab=sessions', '?tab=logs', '?tab=settings'])(
    'CLI-001, UPD-001, D-5: with %j no control is named for a desktop feature',
    async (search) => {
      await show(
        answers({
          Events: NO_EVENTS,
          DeviceSessions: { sessions: { pageInfo: { hasNextPage: false }, edges: [] } },
        }),
        search,
      );
      const controls = [...document.querySelectorAll('button, a, [role="tab"], [role="menuitem"], label')].map(
        (each) => `${each.textContent} ${each.getAttribute('aria-label') ?? ''}`,
      );
      expect(controls.filter((text) => DESKTOP_ONLY.test(text))).toEqual([]);
      expect(panel()).not.toBeNull();
    },
  );

  it.each([
    { name: 'the browser answers', opens: 'a browser tab', platform: BROWSER, browserTabs: 1, webViewTabs: 0 },
    { name: 'the desktop answers', opens: 'a web view tab', platform: DESKTOP_MAC, browserTabs: 0, webViewTabs: 1 },
  ] as const)('CLI-001, WEB-011: with $name the Web button opens $opens', async ({ platform, browserTabs, webViewTabs }) => {
    current.answers = platform;
    const open = vi.spyOn(window, 'open').mockReturnValue(null);
    const web = { id: 'w1', deviceId: ID, label: 'Admin', url: 'http://nas.lan/', openIn: 'ANIMA_TAB' as const, pinnedFingerprintSha256: null, pinnedAt: null };
    await show(answers({ Device: { device: device({ webInterfaces: [web] }) } }));
    document.querySelector<HTMLButtonElement>('button[aria-label="Open Admin of nas"]')?.click();
    await settle();
    expect(open).toHaveBeenCalledTimes(browserTabs);
    expect(current.opened).toHaveLength(webViewTabs);
  });
});
