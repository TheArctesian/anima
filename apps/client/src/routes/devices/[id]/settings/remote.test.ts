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

const RDP_SECRET = 'rdp-hunter2';
const VNC_SECRET = 'vnc-hunter2';
const CERTIFICATE = { fingerprintSha256: 'AA:BB:CC', subject: 'CN=desk', notAfter: null, firstSeenAt: '2026-11-04T09:00:00Z' };

/** The device with RDP and VNC on; each says whether a password is stored. */
function remote(rdpStored: boolean, vncStored: boolean) {
  const base = device();
  return device({
    rdp: { ...base.rdp, enabled: true, username: 'admin', credentialStored: rdpStored },
    vnc: { ...base.vnc, enabled: true, credentialStored: vncStored },
  });
}

function typePassword(protocol: 'rdp' | 'vnc', secret: string): void {
  const input = document.getElementById(`device-${protocol}-password`);
  if (!(input instanceof HTMLInputElement)) throw new Error(`no ${protocol} password input`);
  input.value = secret;
  input.dispatchEvent(new Event('input', { bubbles: true }));
}

const stored = (protocol: 'rdp' | 'vnc') => document.querySelector(`[data-section="${protocol}"]`)?.textContent.includes('Credential stored');
const sent = (fake: ReturnType<typeof fakeHub>, operation: string) => fake.requests.filter((request) => request.operation === operation);

/** Every console call so far, as text. */
function consoleText(spies: readonly ReturnType<typeof vi.spyOn>[]): string {
  return JSON.stringify(spies.map((spy) => spy.mock.calls));
}

describe('settings tab: RDP and VNC passwords (DEV-011, LOG-007)', () => {
  it('DEV-011, LOG-007: typed RDP and VNC passwords are each stored once, the form says so, and no console call holds them', async () => {
    const spies = (['log', 'info', 'warn', 'error', 'debug'] as const).map((level) => vi.spyOn(console, level).mockImplementation(() => undefined));
    const fake = await show(
      answers({
        Device: { device: remote(false, false) },
        SetRdpCredential: { setRdpCredential: { credentialStored: true } },
        SetVncCredential: { setVncCredential: { credentialStored: true } },
      }),
      '?tab=settings&section=rdp',
    );
    typePassword('rdp', RDP_SECRET);
    button('VNC').click();
    await settle();
    typePassword('vnc', VNC_SECRET);
    button('Save').click();
    await settle();
    expect(sent(fake, 'SetRdpCredential')).toEqual([
      { operation: 'SetRdpCredential', variables: { deviceId: ID, credential: { password: RDP_SECRET } } },
    ]);
    expect(sent(fake, 'SetVncCredential')).toEqual([
      { operation: 'SetVncCredential', variables: { deviceId: ID, credential: { password: VNC_SECRET } } },
    ]);
    expect([stored('rdp'), stored('vnc')]).toEqual([true, true]);
    expect(consoleText(spies)).not.toMatch(/hunter2/);
    for (const spy of spies) spy.mockRestore();
  });

  it('DEV-011, LOG-007: a failed RDP password is reported, and the error text never quotes the password', async () => {
    const fake = await show(
      answers({
        Device: { device: remote(false, false) },
        SetRdpCredential: { code: 'VALIDATION', message: `bad password ${RDP_SECRET}`, fields: [{ field: 'password', message: RDP_SECRET }] },
      }),
      '?tab=settings&section=rdp',
    );
    typePassword('rdp', RDP_SECRET);
    button('Save').click();
    await settle();
    expect(sent(fake, 'SetRdpCredential')).toHaveLength(1);
    expect(document.body.textContent).toContain('The hub did not store the password.');
    expect(document.body.textContent).not.toContain(RDP_SECRET);
    expect(stored('rdp')).toBe(false);
  });
});

describe('settings tab: removing RDP and VNC passwords (DEV-011)', () => {
  it('DEV-011: Remove credential clears the stored RDP and VNC passwords through the hub', async () => {
    const fake = await show(
      answers({
        Device: { device: remote(true, true) },
        ClearRdpCredential: { clearRdpCredential: { credentialStored: false } },
        ClearVncCredential: { clearVncCredential: { credentialStored: false } },
      }),
      '?tab=settings&section=rdp',
    );
    button('Remove credential', '[data-section="rdp"]').click();
    await settle();
    button('VNC').click();
    await settle();
    button('Remove credential', '[data-section="vnc"]').click();
    await settle();
    expect(sent(fake, 'ClearRdpCredential')).toEqual([{ operation: 'ClearRdpCredential', variables: { deviceId: ID } }]);
    expect(sent(fake, 'ClearVncCredential')).toEqual([{ operation: 'ClearVncCredential', variables: { deviceId: ID } }]);
    expect([stored('rdp'), stored('vnc')]).toEqual([false, false]);
  });
});

describe('settings tab: RDP server certificate (SEC-007)', () => {
  it('SEC-007: Accept certificate after the RDP test sends the shown fingerprint and Settings shows it accepted', async () => {
    const tested = {
      reachable: true,
      authenticated: false,
      hostKey: null,
      hostKeyChanged: false,
      message: null,
      serverCertificate: CERTIFICATE,
      certificateChanged: false,
      certificateMatched: false,
    };
    const fake = await show(
      answers({
        Device: { device: remote(true, false) },
        TestRemoteConnection: { testConnection: tested },
        AcceptServerCertificate: { acceptServerCertificate: CERTIFICATE },
      }),
      '?tab=settings&section=rdp',
    );
    document.querySelector<HTMLButtonElement>('[data-connection-test="rdp"]')?.click();
    await settle();
    expect(document.querySelector('[aria-label="Server certificate seen"]')?.textContent).toContain('AA:BB:CC');
    button('Accept certificate').click();
    await settle();
    expect(sent(fake, 'AcceptServerCertificate')).toEqual([
      { operation: 'AcceptServerCertificate', variables: { deviceId: ID, fingerprintSha256: 'AA:BB:CC' } },
    ]);
    expect(document.querySelector('[aria-label="Server certificate"]')?.textContent).toContain('AA:BB:CC');
  });
});
