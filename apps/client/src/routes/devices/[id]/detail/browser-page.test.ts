// @vitest-environment jsdom
import { flushSync, mount, tick, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { fakeHub } from '$lib/devices/hub.fixture';
import { endTransitionsAtOnce } from '$lib/devices/transitions.fixture';
import Page from '../+page.svelte';
import { parseDeviceId } from '../device-id';
import { ID, OTHER_ID, answers, device, event, route } from '../page.fixture.svelte';

vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('$lib/motion/engine.fixture')).fakeRun,
}));

const current = vi.hoisted(() => ({ hub: undefined as unknown, opened: [] as unknown[] }));
vi.mock('$lib/hub/context', () => ({ useHub: () => current.hub }));
vi.mock('$lib/sessions/sessions.svelte', () => ({
  useSessions: () => ({ openSession: (request: unknown) => current.opened.push(request), openWeb: (request: unknown) => current.opened.push(request) }),
}));
// The page as a browser runs it: every desktop-only answer is no (CLI-001).
vi.mock('$lib/platform/platform', async () => {
  const { BROWSER } = await import('$lib/platform/answers.fixture');
  return { capabilities: () => BROWSER };
});
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
    'CLI-001, UPD-001, D-5: with %j no desktop-only control renders',
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

  it('CLI-001, WEB-011: the Web button opens a browser tab and never a web view tab', async () => {
    const open = vi.spyOn(window, 'open').mockReturnValue(null);
    const web = { id: 'w1', deviceId: ID, label: 'Admin', url: 'http://nas.lan/', openIn: 'ANIMA_TAB' as const, pinnedFingerprintSha256: null, pinnedAt: null };
    await show(answers({ Device: { device: device({ webInterfaces: [web] }) } }));
    document.querySelector<HTMLButtonElement>('button[aria-label="Open Admin of nas"]')?.click();
    await settle();
    expect(open).toHaveBeenCalledExactlyOnceWith('http://nas.lan/', '_blank', 'noopener,noreferrer');
    expect(current.opened).toEqual([]);
  });
});
