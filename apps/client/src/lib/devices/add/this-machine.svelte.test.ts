// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import type { ThisMachine } from '$lib/desktop/this-machine';
import AddDeviceModal from './AddDeviceModal.svelte';
import { installAnimations } from '../../forms/animations.fixture';
import { button, input, settle, type } from '../form/form/form.fixture';
import { testResult } from '../form/result.fixture';

vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('../../motion/engine.fixture')).fakeRun,
}));

// The Remote Login button shows only where the platform can open System Settings: the desktop app on a Mac (R-059).
const platform = vi.hoisted(() => ({ desktop: true }));
vi.mock('$lib/platform/platform', async () => {
  const { BROWSER, DESKTOP_MAC } = await import('$lib/platform/answers.fixture');
  return { capabilities: () => (platform.desktop ? DESKTOP_MAC : BROWSER) };
});

const MAC: ThisMachine = { name: 'Studio', user: 'stephen', os: 'MACOS', vpnAddress: '100.64.0.7', sshListening: true };

let app: ReturnType<typeof mount> | undefined;

beforeEach(() => {
  installAnimations();
  platform.desktop = true;
});

afterEach(() => {
  if (app) void unmount(app);
  app = undefined;
  document.body.replaceChildren();
  vi.restoreAllMocks();
});

function open(props: Record<string, unknown> = {}) {
  const calls = {
    onCreate: vi.fn(() => Promise.resolve({ ok: true as const, value: { id: 'd9', credentialError: null } })),
    onTest: vi.fn(() =>
      Promise.resolve({ ok: true as const, value: testResult({ reachable: true, authenticated: true, hostKeyChanged: false }) }),
    ),
    onDontAsk: vi.fn(),
  };
  app = mount(AddDeviceModal, {
    target: document.body,
    props: { open: true, sites: [{ id: 's1', name: 'Home', subnet: '10.10.0.0/24' }], onClose: () => undefined, onCreated: () => undefined, ...calls, ...props },
  });
  flushSync();
  return calls;
}

const choice = () => document.querySelector('[role="group"][aria-label="Device to add"]');
const pressed = () => choice()?.querySelector('[aria-pressed="true"]')?.textContent.trim();
const title = () => document.querySelector('[role="dialog"] h2')?.textContent;
const sshBox = () => document.querySelector<HTMLInputElement>('[data-section="ssh"] input[type="checkbox"]');
const remoteLogin = () => document.querySelector('.remote-login')?.textContent ?? '';

it('R-060: with this Mac known, "This Mac" is chosen and every field it knows is filled in', () => {
  open({ machine: MAC });
  expect(pressed()).toBe('This Mac');
  expect(choice()?.textContent).toContain('Another device');
  expect(input('name').value).toBe('Studio');
  expect(input('host').value).toBe('100.64.0.7');
  expect(input('kind').value).toBe('workstation');
  expect(document.querySelector<HTMLSelectElement>('#device-os')?.value).toBe('MACOS');
  expect(sshBox()?.checked).toBe(true);
  expect(input('ssh-port').value).toBe('22');
  expect(input('ssh-username').value).toBe('stephen');
  expect(document.querySelector<HTMLInputElement>('input[type="radio"][value="PASSWORD"]')?.checked).toBe(true);
  expect(input('credential-password').value).toBe('');
});

it('R-060: "Another device" gives the empty form, and "This Mac" fills it again', () => {
  open({ machine: MAC });
  button('Another device').click();
  flushSync();
  expect(pressed()).toBe('Another device');
  expect(input('name').value).toBe('');
  expect(input('host').value).toBe('');
  expect(sshBox()?.checked).toBe(false);
  button('This Mac').click();
  flushSync();
  expect(input('host').value).toBe('100.64.0.7');
});

it('R-060: on Linux the choice says "This computer"', () => {
  open({ machine: { ...MAC, os: 'LINUX' } });
  expect(pressed()).toBe('This computer');
  expect(document.querySelector<HTMLSelectElement>('#device-os')?.value).toBe('LINUX');
});

it('R-060: without a machine to offer there is no choice and the form is empty, as before', () => {
  open({ machine: null });
  expect(choice()).toBeNull();
  expect(title()).toBe('Add device');
  expect(input('name').value).toBe('');
});

it('R-060: the operator types only the password; any field stays editable and is saved as typed', async () => {
  const calls = open({ machine: MAC });
  type('name', 'Studio Mac');
  type('credential-password', 'hunter2');
  button('Save').click();
  await settle();
  expect(calls.onCreate).toHaveBeenCalledOnce();
  expect(calls.onCreate.mock.calls[0]).toEqual([
    expect.objectContaining({
      name: 'Studio Mac',
      host: '100.64.0.7',
      kind: 'workstation',
      os: 'MACOS',
      ssh: { enabled: true, port: '22', username: 'stephen', auth: 'PASSWORD' },
      credential: expect.objectContaining({ password: 'hunter2' }) as unknown,
    }),
    { kind: 'existing', id: 's1' },
  ]);
});

it('DEV-010, DEV-013: the pre-filled form still validates and tests the connection', async () => {
  const calls = open({ machine: MAC });
  type('host', 'not a host!');
  button('Save').click();
  await settle();
  expect(calls.onCreate).not.toHaveBeenCalled();
  expect(document.querySelector('[aria-invalid="true"]')?.id).toBe('device-host');
  type('host', '100.64.0.7');
  button('Test connection').click();
  await settle();
  expect(calls.onTest).toHaveBeenCalledWith(expect.objectContaining({ host: '100.64.0.7', ssh: expect.objectContaining({ username: 'stephen' }) as unknown }));
});

it('R-060: when SSH did not answer on this Mac, SSH is off and the Remote Login button says why', () => {
  open({ machine: { ...MAC, sshListening: false } });
  expect(sshBox()?.checked).toBe(false);
  expect(document.querySelector('.remote-login [role="status"]')?.textContent).toContain('SSH is off');
  expect(document.querySelector('[data-section="ssh"]')?.textContent).toContain('Open Remote Login settings');
  button('Another device').click();
  flushSync();
  expect(remoteLogin()).not.toContain('SSH is off');
  expect(remoteLogin()).toContain('Open Remote Login settings');
});

it('R-060: with SSH answering, the Remote Login button keeps its usual text', () => {
  open({ machine: MAC });
  expect(remoteLogin()).not.toContain('SSH is off');
});

it('R-059, CLI-001: with the browser answers no Remote Login button is offered', () => {
  platform.desktop = false;
  open({ machine: { ...MAC, sshListening: false } });
  expect(document.querySelector('.remote-login')).toBeNull();
  expect(document.body.textContent).not.toContain('Open Remote Login settings');
});

it('R-060: asked by the app, the title is "Add this Mac?" and "Don\'t ask again on this Mac" is offered', () => {
  const calls = open({ machine: MAC, asking: true });
  expect(title()).toBe('Add this Mac?');
  button("Don't ask again on this Mac").click();
  expect(calls.onDontAsk).toHaveBeenCalledOnce();
});

it('R-060: opened from Add device, the title stays "Add device" and there is no "Don\'t ask again"', () => {
  open({ machine: MAC });
  expect(title()).toBe('Add device');
  expect(document.body.textContent).not.toContain("Don't ask again");
});
