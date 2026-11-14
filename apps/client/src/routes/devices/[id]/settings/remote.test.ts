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

/** Types into a protocol's password field; SSH's is the device credential's. */
function typePassword(protocol: 'ssh' | 'rdp' | 'vnc', secret: string): void {
  const input = document.getElementById(protocol === 'ssh' ? 'device-credential-password' : `device-${protocol}-password`);
  if (!(input instanceof HTMLInputElement)) throw new Error(`no ${protocol} password input`);
  input.value = secret;
  input.dispatchEvent(new Event('input', { bubbles: true }));
}

const stored = (protocol: 'ssh' | 'rdp' | 'vnc') => document.querySelector(`[data-connection="${protocol}"]`)?.textContent.includes('in the hub, encrypted');

/** Opens a protocol's row when it is closed. */
async function openRow(protocol: 'rdp' | 'vnc'): Promise<void> {
  const toggle = document.querySelector<HTMLButtonElement>(`[data-connection="${protocol}"] [aria-expanded]`);
  if (toggle === null) throw new Error(`no ${protocol} row`);
  if (toggle.getAttribute('aria-expanded') !== 'true') toggle.click();
  await settle();
}

/** D17: a password is typed into a field of its own and stored by Save password. */
async function storePassword(protocol: 'rdp' | 'vnc', secret: string): Promise<void> {
  await openRow(protocol);
  button('Add password', `[data-connection="${protocol}"]`).click();
  await settle();
  typePassword(protocol, secret);
  button('Save password', `[data-connection="${protocol}"]`).click();
  await settle();
}
const sent = (fake: ReturnType<typeof fakeHub>, operation: string) => fake.requests.filter((request) => request.operation === operation);

/** Every console call so far, as text. */
function consoleText(spies: readonly ReturnType<typeof vi.spyOn>[]): string {
  return JSON.stringify(spies.map((spy) => spy.mock.calls));
}

describe('settings tab: RDP and VNC passwords (DEV-011, LOG-007)', () => {
  it('DEV-011, LOG-007, D17: RDP and VNC passwords typed and saved are each stored once, the rows say so, and no console call holds them', async () => {
    const spies = (['log', 'info', 'warn', 'error', 'debug'] as const).map((level) => vi.spyOn(console, level).mockImplementation(() => undefined));
    const fake = await show(
      answers({
        Device: { device: remote(false, false) },
        SetRdpCredential: { setRdpCredential: { credentialStored: true } },
        SetVncCredential: { setVncCredential: { credentialStored: true } },
      }),
      '?tab=settings&section=rdp',
    );
    await storePassword('rdp', RDP_SECRET);
    await storePassword('vnc', VNC_SECRET);
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
    await storePassword('rdp', RDP_SECRET);
    expect(sent(fake, 'SetRdpCredential')).toHaveLength(1);
    expect(document.body.textContent).toContain('The hub did not store the password.');
    expect(document.body.textContent).not.toContain(RDP_SECRET);
    expect(stored('rdp')).toBe(false);
  });
});

describe('settings tab: replacing and removing stored secrets (D17, D-4, DEV-011, DEV-012)', () => {
  it('D-4: Remove clears the stored SSH credential, and the row no longer says one is stored', async () => {
    const fake = await show(answers({ ClearSshCredential: { clearSshCredential: { credentialStored: false } } }), '?tab=settings');
    button('Remove', '[data-connection="ssh"]').click();
    await settle();
    expect(fake.requests.at(-1)).toEqual({ operation: 'ClearSshCredential', variables: { deviceId: ID } });
    expect(stored('ssh')).toBe(false);
  });

  it('D17, DEV-012: Replace opens a field of its own; Cancel sends nothing, Save password stores it once and the field goes', async () => {
    const fake = await show(answers({ SetSshCredential: { setSshCredential: { credentialStored: true } } }), '?tab=settings');
    button('Replace', '[data-connection="ssh"]').click();
    await settle();
    typePassword('ssh', 'hunter2');
    button('Cancel', '[data-connection="ssh"]').click();
    await settle();
    expect(document.getElementById('device-credential-password')).toBeNull();
    button('Replace', '[data-connection="ssh"]').click();
    await settle();
    expect((document.getElementById('device-credential-password') as HTMLInputElement | null)?.value).toBe('');
    typePassword('ssh', 'hunter2');
    button('Save password', '[data-connection="ssh"]').click();
    await settle();
    expect(sent(fake, 'SetSshCredential')).toEqual([
      { operation: 'SetSshCredential', variables: { deviceId: ID, credential: { password: 'hunter2' } } },
    ]);
    expect(sent(fake, 'UpdateDevice')).toEqual([]);
    expect(document.getElementById('device-credential-password')).toBeNull();
  });

  it('DEV-011: Remove clears the stored RDP and VNC passwords through the hub', async () => {
    const fake = await show(
      answers({
        Device: { device: remote(true, true) },
        ClearRdpCredential: { clearRdpCredential: { credentialStored: false } },
        ClearVncCredential: { clearVncCredential: { credentialStored: false } },
      }),
      '?tab=settings&section=rdp',
    );
    await openRow('rdp');
    button('Remove', '[data-connection="rdp"]').click();
    await settle();
    await openRow('vnc');
    button('Remove', '[data-connection="vnc"]').click();
    await settle();
    expect(sent(fake, 'ClearRdpCredential')).toEqual([{ operation: 'ClearRdpCredential', variables: { deviceId: ID } }]);
    expect(sent(fake, 'ClearVncCredential')).toEqual([{ operation: 'ClearVncCredential', variables: { deviceId: ID } }]);
    expect([stored('rdp'), stored('vnc')]).toEqual([false, false]);
  });
});

/** The RDP test's answer with this certificate; `changed` says the hub held another one. */
function certificateTest(serverCertificate: typeof CERTIFICATE, changed: boolean) {
  return {
    reachable: true,
    authenticated: false,
    hostKey: null,
    hostKeyChanged: false,
    message: null,
    serverCertificate,
    certificateChanged: changed,
    certificateMatched: false,
  };
}

/** Runs the RDP connection test, then presses Accept certificate. */
async function testAndAccept(): Promise<void> {
  document.querySelector<HTMLButtonElement>('[data-connection-test="rdp"]')?.click();
  await settle();
  button('Accept certificate').click();
  await settle();
}

describe('settings tab: RDP server certificate (SEC-007)', () => {
  it('SEC-007: Accept certificate after the RDP test sends the shown fingerprint and Settings shows it accepted', async () => {
    const fake = await show(
      answers({
        Device: { device: remote(true, false) },
        TestRemoteConnection: { testConnection: certificateTest(CERTIFICATE, false) },
        AcceptServerCertificate: { acceptServerCertificate: CERTIFICATE },
      }),
      '?tab=settings&section=rdp',
    );
    await testAndAccept();
    expect(sent(fake, 'AcceptServerCertificate')).toEqual([
      { operation: 'AcceptServerCertificate', variables: { deviceId: ID, fingerprintSha256: 'AA:BB:CC' } },
    ]);
    expect(document.querySelector('[aria-label="Server certificate"]')?.textContent).toContain('AA:BB:CC');
  });

  it('SEC-007: with a certificate already pinned, accepting a changed one sends the new fingerprint, not the pinned one', async () => {
    const changed = { ...CERTIFICATE, fingerprintSha256: 'DD:EE:FF', subject: 'CN=desk-new' };
    const pinned = remote(true, false);
    const fake = await show(
      answers({
        Device: { device: { ...pinned, rdp: { ...pinned.rdp, serverCertificate: CERTIFICATE } } },
        TestRemoteConnection: { testConnection: certificateTest(changed, true) },
        AcceptServerCertificate: { acceptServerCertificate: changed },
      }),
      '?tab=settings&section=rdp',
    );
    expect(document.body.textContent).not.toContain('DD:EE:FF');
    await testAndAccept();
    expect(sent(fake, 'AcceptServerCertificate')).toEqual([
      { operation: 'AcceptServerCertificate', variables: { deviceId: ID, fingerprintSha256: 'DD:EE:FF' } },
    ]);
    const shown = document.querySelector('[aria-label="Server certificate"]')?.textContent ?? '';
    expect(shown).toContain('DD:EE:FF');
    expect(shown).not.toContain('AA:BB:CC');
  });
});
