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

/** The input with this id. */
function field(id: string): HTMLInputElement {
  const found = document.getElementById(id);
  if (!(found instanceof HTMLInputElement)) throw new Error(`no input ${id}`);
  return found;
}

/** Types `text` into the input, as each key press would leave it. */
function type(id: string, text: string): void {
  field(id).value = text;
  field(id).dispatchEvent(new Event('input', { bubbles: true }));
}

/** Leaves the input, as Tab or a click elsewhere would. */
function leave(id: string): void {
  field(id).dispatchEvent(new FocusEvent('blur'));
}

const sent = (fake: ReturnType<typeof fakeHub>, operation: string) => fake.requests.filter((request) => request.operation === operation);

/** The SSH row's switch. */
function sshSwitch(): HTMLButtonElement {
  const found = document.querySelector<HTMLButtonElement>('[data-connection="ssh"] [role="switch"]');
  if (found === null) throw new Error('no SSH switch');
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

  it('D17, D-2: a typed name is not sent while typing; leaving the field saves it once and the header shows it', async () => {
    const fake = await show(answers({ UpdateDevice: { updateDevice: device({ name: 'vault' }) } }), '?tab=settings');
    type('device-name', 'vau');
    await settle();
    expect(sent(fake, 'UpdateDevice')).toEqual([]);
    type('device-name', 'vault');
    leave('device-name');
    await settle();
    leave('device-name');
    await settle();
    expect(sent(fake, 'UpdateDevice')).toEqual([{ operation: 'UpdateDevice', variables: { id: ID, input: { name: 'vault' } } }]);
    expect(document.querySelector('header h1')?.textContent).toBe('vault');
  });

  it('D17, T3/T5: Return saves a text field, and the test of the saved host then goes by ID', async () => {
    const moved = device({ host: 'vault.home.arpa' });
    const tested = { reachable: true, authenticated: true, hostKey: null, hostKeyChanged: false, message: null };
    const fake = await show(answers({ UpdateDevice: { updateDevice: moved }, TestConnection: { testConnection: tested } }), '?tab=settings');
    type('device-host', 'vault.home.arpa');
    field('device-host').dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
    await settle();
    expect(sent(fake, 'UpdateDevice')).toEqual([{ operation: 'UpdateDevice', variables: { id: ID, input: { host: 'vault.home.arpa' } } }]);
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

describe('settings tab: saving on change (D17, DEV-010, D-2, SEC-006)', () => {
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

  it('D17: the SSH switch saves at once, with the rest of the SSH settings as the hub input needs them', async () => {
    const off = device({}, { enabled: false });
    const fake = await show(answers({ UpdateDevice: { updateDevice: off } }), '?tab=settings');
    sshSwitch().click();
    await settle();
    expect(sent(fake, 'UpdateDevice')).toEqual([
      { operation: 'UpdateDevice', variables: { id: ID, input: { ssh: { enabled: false, port: 22, username: 'root', auth: 'PASSWORD' } } } },
    ]);
    expect(sshSwitch().getAttribute('aria-checked')).toBe('false');
  });

  it('D17, DEV-010: a port that is not a port is never sent; it stays typed and outlined, and a line under the fields says why', async () => {
    const fake = await show(answers(), '?tab=settings');
    type('device-ssh-port', 'abc');
    leave('device-ssh-port');
    await settle();
    expect(sent(fake, 'UpdateDevice')).toEqual([]);
    expect(field('device-ssh-port').value).toBe('abc');
    expect(field('device-ssh-port').getAttribute('aria-invalid')).toBe('true');
    expect(document.getElementById('device-ssh-port-error')?.textContent).toBe('Enter a port from 1 to 65535.');
  });

  it('D17, D-2: a value the hub refuses stays shown and unsaved, outlined, with the reason the hub gave under it', async () => {
    const refusal = { code: 'VALIDATION', message: 'Invalid input', fields: [{ field: 'name', message: 'Another device has this name.' }] };
    await show(answers({ UpdateDevice: refusal }), '?tab=settings');
    type('device-name', 'atlas');
    leave('device-name');
    await settle();
    expect(field('device-name').value).toBe('atlas');
    expect(field('device-name').getAttribute('aria-invalid')).toBe('true');
    expect(document.getElementById('device-name-error')?.textContent).toBe('Another device has this name.');
    expect(document.querySelector('header h1')?.textContent).toBe('nas');
  });

});

describe('settings tab: opening a section (DEV-022)', () => {
  it('DEV-022: a header button with Settings already open opens its section, and the no-key SSH focuses Test connection', async () => {
    await show(answers(), '?tab=settings&section=ssh');
    const rdp = document.querySelector<HTMLButtonElement>('[data-connection="rdp"] [aria-expanded]');
    if (rdp === null) throw new Error('no RDP row');
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
