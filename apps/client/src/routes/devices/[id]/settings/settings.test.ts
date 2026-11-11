// @vitest-environment jsdom
import { flushSync, mount, tick, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { fakeHub } from '$lib/devices/hub.fixture';
import { endTransitionsAtOnce } from '$lib/devices/transitions.fixture';
import Page from '../+page.svelte';
import { parseDeviceId } from '../device-id';
import type { WebInterface } from '$lib/web-interfaces/api';
import { ID, KEY, answers, device, route } from '../page.fixture.svelte';

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

  it('DEV-014: a failed delete shows the hub message and stays on the page', async () => {
    await show(answers({ DeleteDevice: { code: 'UNAVAILABLE', message: 'The database is unreachable.' } }), '?tab=settings');
    button('Delete device').click();
    await settle();
    button('Delete nas', '[role="dialog"]').click();
    await settle();
    expect(document.querySelector('[role="dialog"] [role="alert"]')?.textContent).toContain('The database is unreachable.');
    expect(current.goto).toEqual([]);
  });

});

describe('settings tab: the agent (MET-019, D-7)', () => {
  it('MET-019: the Settings tab offers Install agent, and its code request names this device', async () => {
    const enrolment = { enrolAgent: { code: 'anima_enrol_AbCdEfGhIjKlMnOpQrStUv', expiresAt: '2099-01-01T00:00:00Z' } };
    const fake = await show(answers({ EnrolAgent: enrolment }), '?tab=settings');
    button('Install agent').click();
    await settle();
    expect(fake.requests.filter((request) => request.operation === 'EnrolAgent')).toEqual([
      { operation: 'EnrolAgent', variables: { deviceId: ID } },
    ]);
    expect(document.querySelector('pre')?.textContent).toContain('anima-agent enrol --hub');
  });
});

describe('settings tab: credentials, cancel and sections (D-4, DEV-022, SEC-006)', () => {
  it('SEC-006: accepting the new host key makes SSH start sessions', async () => {
    const tested = { reachable: true, authenticated: true, hostKey: KEY, hostKeyChanged: false, message: null };
    await show(
      answers({ Device: { device: device({}, { hostKey: null }) }, TestConnection: { testConnection: tested }, AcceptHostKey: { acceptHostKey: KEY } }),
      '?tab=settings',
    );
    button('Test connection').click();
    await settle();
    button('Accept', '[role="dialog"]').click();
    await settle();
    expect(button('SSH to nas')).not.toBeNull();
  });

  it('D-4: removing the credential leaves the form without a stored one', async () => {
    const fake = await show(answers({ ClearSshCredential: { clearSshCredential: { credentialStored: false } } }), '?tab=settings');
    button('Remove credential').click();
    await settle();
    expect(fake.requests.at(-1)).toEqual({ operation: 'ClearSshCredential', variables: { deviceId: ID } });
    expect(document.body.textContent).not.toContain('Credential stored');
  });

  it('D-2: Cancel puts the saved values back', async () => {
    await show(answers(), '?tab=settings');
    const name = document.getElementById('device-name');
    if (!(name instanceof HTMLInputElement)) throw new Error('no name input');
    name.value = 'vault';
    name.dispatchEvent(new Event('input', { bubbles: true }));
    button('Cancel').click();
    await settle();
    expect((document.getElementById('device-name') as HTMLInputElement | null)?.value).toBe('nas');
  });

  it('DEV-022: a header button with Settings already open opens its section, and the no-key SSH focuses Test connection', async () => {
    await show(answers(), '?tab=settings&section=ssh');
    const rdp = button('Remote Desktop (RDP)');
    expect(rdp.getAttribute('aria-expanded')).toBe('false');
    route.search = '?tab=settings&section=rdp';
    await settle();
    expect(rdp.getAttribute('aria-expanded')).toBe('true');
    expect(document.activeElement).toBe(rdp);
    route.search = '?tab=settings&section=test';
    await settle();
    expect(document.activeElement).toBe(button('Test connection'));
  });
});

const PVE: WebInterface = { id: 'w1', deviceId: ID, label: 'Proxmox VE', url: 'https://pve.lan:8006', openIn: 'ANIMA_TAB', pinnedFingerprintSha256: null, pinnedAt: null };
const GRAFANA = { checkId: 'c9', url: 'https://grafana.lan:3000', label: 'grafana.lan' };
const rows = () => [...document.querySelectorAll('li[aria-label]')].map((row) => row.getAttribute('aria-label'));

describe('settings tab: web interfaces (WEB-001)', () => {

  it('WEB-001: Settings shows the Web interfaces section; Add stores the interface and the list shows it', async () => {
    const fake = await show(answers({ CreateWebInterface: { createWebInterface: PVE } }), '?tab=settings');
    expect(document.getElementById('web-interfaces-heading')?.textContent).toBe('Web interfaces');
    button('Add a web interface').click();
    await settle();
    const url = document.getElementById('web-new-url');
    if (!(url instanceof HTMLInputElement)) throw new Error('no URL input');
    url.value = PVE.url;
    url.dispatchEvent(new Event('input', { bubbles: true }));
    button('Add web interface').click();
    await settle();
    expect(fake.requests.filter((request) => request.operation === 'CreateWebInterface')).toEqual([
      { operation: 'CreateWebInterface', variables: { deviceId: ID, input: { label: null, url: PVE.url, openIn: 'ANIMA_TAB' } } },
    ]);
    expect(rows()).toEqual(['Proxmox VE']);
  });

  it('WEB-001: saving a changed row updates the interface once and the list shows the stored label', async () => {
    const fake = await show(
      answers({ Device: { device: device({ webInterfaces: [PVE] }) }, UpdateWebInterface: { updateWebInterface: { ...PVE, label: 'PVE' } } }),
      '?tab=settings',
    );
    const label = document.getElementById('web-w1-label');
    if (!(label instanceof HTMLInputElement)) throw new Error('no label input');
    label.value = 'PVE';
    label.dispatchEvent(new Event('input', { bubbles: true }));
    button('Save', 'li[aria-label="Proxmox VE"]').click();
    await settle();
    expect(fake.requests.filter((request) => request.operation === 'UpdateWebInterface')).toEqual([
      { operation: 'UpdateWebInterface', variables: { id: 'w1', input: { label: 'PVE', url: PVE.url, openIn: 'ANIMA_TAB' } } },
    ]);
    expect(rows()).toEqual(['PVE']);
  });

  it('WEB-001: a confirmed delete removes the interface from the list', async () => {
    const fake = await show(
      answers({ Device: { device: device({ webInterfaces: [PVE] }) }, DeleteWebInterface: { deleteWebInterface: 'w1' } }),
      '?tab=settings',
    );
    button('Delete', 'li[aria-label="Proxmox VE"]').click();
    await settle();
    button('Delete Proxmox VE', '[role="dialog"]').click();
    await settle();
    expect(fake.requests.filter((request) => request.operation === 'DeleteWebInterface')).toEqual([
      { operation: 'DeleteWebInterface', variables: { id: 'w1' } },
    ]);
    expect(rows()).toEqual([]);
  });

});

describe('settings tab: web interface suggestions (WEB-002)', () => {
  it('WEB-002: accepting a suggestion stores it once, the list shows it and the suggestion goes', async () => {
    const offered = device({ webInterfaceSuggestions: [GRAFANA] });
    const made = { ...PVE, id: 'w2', label: 'grafana.lan', url: GRAFANA.url };
    const fake = await show(
      answers({ Device: { device: offered }, AcceptWebInterfaceSuggestion: { acceptWebInterfaceSuggestion: made } }),
      '?tab=settings',
    );
    button('Add grafana.lan').click();
    await settle();
    expect(fake.requests.filter((request) => request.operation === 'AcceptWebInterfaceSuggestion')).toEqual([
      { operation: 'AcceptWebInterfaceSuggestion', variables: { checkId: 'c9' } },
    ]);
    expect(rows()).toEqual(['grafana.lan']);
    expect(document.getElementById('web-suggestions-heading')).toBeNull();
  });

  it('WEB-002: dismissing a suggestion removes it without adding an interface', async () => {
    const fake = await show(
      answers({ Device: { device: device({ webInterfaceSuggestions: [GRAFANA] }) }, DismissWebInterfaceSuggestion: { dismissWebInterfaceSuggestion: 'c9' } }),
      '?tab=settings',
    );
    button('Dismiss grafana.lan').click();
    await settle();
    expect(fake.requests.at(-1)).toEqual({ operation: 'DismissWebInterfaceSuggestion', variables: { checkId: 'c9' } });
    expect(rows()).toEqual([]);
    expect(document.getElementById('web-suggestions-heading')).toBeNull();
  });
});
