// @vitest-environment jsdom
import { flushSync, mount, tick, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { fakeHub } from '$lib/devices/hub.fixture';
import { endTransitionsAtOnce } from '$lib/devices/transitions.fixture';
import Page from '../+page.svelte';
import { parseDeviceId } from '../device-id';
import { ID, answers, device, route } from '../page.fixture.svelte';

vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('$lib/motion/engine.fixture')).fakeRun,
}));

const current = vi.hoisted(() => ({ hub: undefined as unknown, goto: [] as [string, unknown][], opened: [] as unknown[] }));
vi.mock('$lib/hub/context', () => ({ useHub: () => current.hub }));
vi.mock('$lib/sessions/sessions.svelte', () => ({
  useSessions: () => ({ openSession: (request: unknown) => current.opened.push(request) }),
}));
vi.mock('$app/navigation', () => ({
  goto: (href: string, options: unknown) => {
    current.goto.push([href, options]);
    return Promise.resolve();
  },
}));
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

/** The button with this text (icon glyphs left out) or this accessible label, inside `within` when given. */
function button(label: string, within = 'body'): HTMLButtonElement {
  const found = [...document.querySelectorAll<HTMLButtonElement>(`${within} button`)].find(
    (candidate) => candidate.textContent.replace(/\p{Co}/gu, '').trim() === label || candidate.getAttribute('aria-label') === label,
  );
  if (!found) throw new Error(`no button ${label}`);
  return found;
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

describe('settings tab (D-7, DEV-014)', () => {
  it('DEV-014: Cancel and Escape do not delete; confirming deletes once and goes to the list', async () => {
    const fake = await show(answers({ DeleteDevice: { deleteDevice: ID } }), '?tab=settings');
    button('Delete device').click();
    await settle();
    expect(document.querySelector('[role="dialog"]')?.textContent).toContain('credentials, checks, heartbeats');
    button('Cancel', '[role="dialog"]').click();
    await settle();
    button('Delete device').click();
    await settle();
    document.querySelector('[role="dialog"]')?.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
    await settle();
    expect(fake.requests.some((request) => request.operation === 'DeleteDevice')).toBe(false);

    button('Delete device').click();
    await settle();
    button('Delete nas', '[role="dialog"]').click();
    await settle();
    expect(fake.requests.filter((request) => request.operation === 'DeleteDevice')).toEqual([{ operation: 'DeleteDevice', variables: { id: ID } }]);
    expect(current.goto).toEqual([['/devices', undefined]]);
  });

  it('D-2: a save sends only the changed field, the header shows the saved name, and saving again sends nothing', async () => {
    const renamed = device({ name: 'vault' });
    const fake = await show(answers({ UpdateDevice: { updateDevice: renamed } }), '?tab=settings');
    const name = document.getElementById('device-name');
    if (!(name instanceof HTMLInputElement)) throw new Error('no name input');
    name.value = 'vault';
    name.dispatchEvent(new Event('input', { bubbles: true }));
    button('Save').click();
    await settle();
    button('Save').click();
    await settle();
    expect(fake.requests.filter((request) => request.operation === 'UpdateDevice')).toEqual([
      { operation: 'UpdateDevice', variables: { id: ID, input: { name: 'vault' } } },
    ]);
    expect(document.querySelector('header h1')?.textContent).toBe('vault');
  });

  it('T3/T5: after a save the form compares with the saved device, so the test of the saved host goes by ID', async () => {
    const moved = device({ host: 'vault.home.arpa' });
    const tested = { reachable: true, authenticated: true, hostKey: null, hostKeyChanged: false, message: null };
    const fake = await show(answers({ UpdateDevice: { updateDevice: moved }, TestConnection: { testConnection: tested } }), '?tab=settings');
    const host = document.getElementById('device-host');
    if (!(host instanceof HTMLInputElement)) throw new Error('no host input');
    host.value = 'vault.home.arpa';
    host.dispatchEvent(new Event('input', { bubbles: true }));
    button('Save').click();
    await settle();
    button('Test connection').click();
    await settle();
    expect(fake.requests.find((request) => request.operation === 'TestConnection')?.variables).toEqual({ input: { deviceId: ID } });
  });
});
