// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { RemoteProtocol } from '../../api';
import { fakeHub } from '../../fake/hub.fixture';
import { installBrowserFakes } from '../view.fixture.svelte';
import { fakeArgumentClient } from './argv.fixture';
import CredentialPrompt from './CredentialPrompt.svelte';
import { askedFields } from './fields';

const PASSWORD = 'PASSWORDMARKER-hunter2';
let cleanups: (() => unknown)[] = [];

beforeEach(() => {
  cleanups = [installBrowserFakes()];
});

afterEach(async () => {
  for (const cleanup of cleanups.reverse()) await cleanup();
  document.body.innerHTML = '';
  vi.restoreAllMocks();
});

/** Let the fake hub's answers arrive, then flush Svelte's updates. */
async function settle() {
  for (let turn = 0; turn < 5; turn += 1) await new Promise((resolve) => setTimeout(resolve, 0));
  flushSync();
}

function showPrompt(protocol: RemoteProtocol, required: string[], refuseCredential = false) {
  const fake = fakeHub({ refuseCredential });
  const argv = fakeArgumentClient();
  const heard = { sent: 0, cancelled: 0 };
  const fields = askedFields(protocol, required);
  if (fields === undefined) throw new Error('nothing to ask');
  const component = mount(CredentialPrompt, {
    target: document.body,
    props: {
      device: { id: 'device-1', name: 'desk' },
      protocol,
      fields,
      client: argv.client,
      api: fake.api,
      onsent: () => (heard.sent += 1),
      oncancel: () => (heard.cancelled += 1),
    },
  });
  flushSync();
  cleanups.push(() => fake.hub.close(), () => unmount(component));
  return { ...fake, argv, heard };
}

const dialog = () => document.querySelector<HTMLElement>('[role="dialog"]');
const input = (name: string) => dialog()?.querySelector<HTMLInputElement>(`input[name="${name}"]`) ?? null;
const saveBox = () => dialog()?.querySelector<HTMLInputElement>('input[type="checkbox"]') ?? null;
const button = (name: string) =>
  [...(dialog()?.querySelectorAll('button') ?? [])].find((candidate) => candidate.textContent.trim() === name);

function type(name: string, value: string) {
  const field = input(name);
  if (field === null) throw new Error(`no ${name} field`);
  field.value = value;
  field.dispatchEvent(new Event('input', { bubbles: true }));
  flushSync();
}

async function submit(save = false) {
  if (save) saveBox()?.click();
  flushSync();
  button('Sign in')?.click();
  await settle();
}

describe('credential prompt (GW-015)', () => {
  it('GW-015: shows only the asked fields, labelled; the password is masked and not autofilled', () => {
    showPrompt('RDP', ['password', 'username']);
    expect([...(dialog()?.querySelectorAll('label span') ?? [])].map((span) => span.textContent)).toEqual(['Username', 'Password']);
    expect(input('domain')).toBeNull();
    expect(input('password')?.type).toBe('password');
    expect(input('password')?.autocomplete).toBe('off');
    expect(dialog()?.getAttribute('aria-modal')).toBe('true');
    expect(document.activeElement).toBe(input('username'));
  });

  it('GW-015: Submit sends each value to its argv stream once, and calls no mutation without Save', async () => {
    const { argv, heard, sent } = showPrompt('RDP', ['username', 'password']);
    expect(saveBox()?.checked).toBe(false);
    type('username', 'ana');
    type('password', PASSWORD);
    await submit();
    button('Sign in')?.click();
    await settle();
    expect(argv.sent()).toEqual([
      { mimetype: 'text/plain', name: 'username', text: 'ana', ends: 1 },
      { mimetype: 'text/plain', name: 'password', text: PASSWORD, ends: 1 },
    ]);
    expect(sent).toEqual([]);
    expect(heard).toEqual({ sent: 1, cancelled: 0 });
  });

  it.each([
    ['RDP', 'SetRdpCredential'],
    ['VNC', 'SetVncCredential'],
  ] as const)('GW-015: with Save checked, %s stores the password only, through %s', async (protocol, operation) => {
    const { argv, heard, named, sent } = showPrompt(protocol, ['username', 'password']);
    type('username', 'ana');
    type('password', PASSWORD);
    await submit(true);
    expect(argv.sent().map(({ name }) => name)).toEqual(['username', 'password']);
    expect(sent.map((request) => request.operation)).toEqual([operation]);
    expect(named(operation)[0]?.variables).toEqual({ deviceId: 'device-1', credential: { password: PASSWORD } });
    expect(heard.sent).toBe(1);
  });

  it('no Save is offered when no password is asked', () => {
    showPrompt('RDP', ['username']);
    expect(saveBox()).toBeNull();
  });

  it('Cancel, and Escape, send nothing and ask the caller to end the session', () => {
    const { argv, heard, sent } = showPrompt('VNC', ['password']);
    type('password', PASSWORD);
    button('Cancel')?.click();
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true }));
    expect(argv.sent()).toEqual([]);
    expect(sent).toEqual([]);
    expect(heard).toEqual({ sent: 0, cancelled: 2 });
  });
});

describe('credential prompt privacy (GW-015, LOG-007)', () => {
  it('LOG-007: a refused save shows fixed text, never the hub message, and the session goes on', async () => {
    const logs = Object.keys(console).map((method) => vi.spyOn(console, method as 'log').mockImplementation(() => undefined));
    const { argv, heard } = showPrompt('RDP', ['password'], true);
    type('password', PASSWORD);
    await submit(true);
    expect(argv.sent()).toHaveLength(1);
    expect(dialog()?.textContent).toContain('did not save the password');
    expect(document.body.innerHTML).not.toContain(PASSWORD);
    expect(heard.sent).toBe(0);
    button('Close')?.click();
    expect(heard).toEqual({ sent: 1, cancelled: 0 });
    expect(logs.flatMap((spy) => spy.mock.calls).join(' ')).not.toContain(PASSWORD);
  });

  it('GW-015: the values are cleared from the prompt once sent', async () => {
    const { hub } = showPrompt('RDP', ['username', 'password']);
    let stop: () => void = () => undefined;
    // Hold the save open, so the prompt is still mounted while the password is on its way.
    vi.spyOn(hub, 'request').mockImplementation(() => new Promise((resolve) => (stop = () => resolve({ ok: false, error: { code: 'unreachable', message: '' } }))));
    type('username', 'ana');
    type('password', PASSWORD);
    await submit(true);
    expect(dialog()?.textContent).toContain('Saving');
    expect(document.body.innerHTML).not.toContain(PASSWORD);
    expect([...document.querySelectorAll('input')].map((field) => field.value)).not.toContain(PASSWORD);
    stop();
  });
});
