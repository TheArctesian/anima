// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { RemoteProtocol, SessionsApi } from '../../api';
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

/** The prompt over a fake hub; `refuseCredential` makes the hub refuse the save, `setCredential` replaces it. */
function showPrompt(protocol: RemoteProtocol, required: string[], hub: { refuseCredential?: boolean } & Partial<SessionsApi> = {}) {
  const { refuseCredential = false, ...calls } = hub;
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
      api: { ...fake.api, ...calls },
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

/** A save that answers only when the test calls `answer`, so the prompt can be seen while it waits. */
function heldSave() {
  const held: { calls: number; answer: (stored: boolean) => void } = { calls: 0, answer: () => undefined };
  const setCredential: SessionsApi['setCredential'] = () => {
    held.calls += 1;
    return new Promise((resolve) => {
      held.answer = (stored) => resolve({ ok: true, value: { credentialStored: stored } });
    });
  };
  return { held, setCredential };
}

const submitEvent = () => new SubmitEvent('submit', { bubbles: true, cancelable: true });

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
    // Two submits before anything settles: the second finds the prompt no longer asking.
    const form = dialog()?.querySelector('form');
    form?.dispatchEvent(submitEvent());
    form?.dispatchEvent(submitEvent());
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
    (document.activeElement ?? document.body).dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true }));
    expect(argv.sent()).toEqual([]);
    expect(sent).toEqual([]);
    expect(heard).toEqual({ sent: 0, cancelled: 2 });
  });
});

describe('credential prompt privacy (GW-015, LOG-007)', () => {
  it('LOG-007: a refused save shows fixed text, never the hub message, and the session goes on', async () => {
    const logs = Object.keys(console).map((method) => vi.spyOn(console, method as 'log').mockImplementation(() => undefined));
    const { argv, heard } = showPrompt('RDP', ['password'], { refuseCredential: true });
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

});

describe('credential prompt after Submit (GW-015)', () => {
  it('GW-015: once sent, the fields are empty, Save is unchecked and focus is on the status, not the page', async () => {
    const { held, setCredential } = heldSave();
    showPrompt('RDP', ['username', 'password'], { setCredential });
    type('username', 'ana');
    type('password', PASSWORD);
    await submit(true);
    expect(held.calls).toBe(1);
    expect(input('username')?.value).toBe('');
    expect(input('password')?.value).toBe('');
    expect(saveBox()?.checked).toBe(false);
    expect(input('password')?.matches(':disabled')).toBe(true);
    expect(document.body.innerHTML).not.toContain(PASSWORD);
    expect(document.activeElement?.textContent.trim()).toBe('Saving the password…');
    held.answer(true);
    await settle();
  });

  it('GW-015: without Save the fields are emptied too, and focus is on the status', async () => {
    showPrompt('VNC', ['username', 'password']);
    type('username', 'ana');
    type('password', PASSWORD);
    await submit();
    expect([input('username')?.value, input('password')?.value]).toEqual(['', '']);
    expect(document.activeElement?.textContent.trim()).toBe('Signing in…');
  });

  it('GW-015: with Save checked and no password typed, nothing is stored and the empty value is sent', async () => {
    const { held, setCredential } = heldSave();
    const { argv, heard } = showPrompt('RDP', ['username', 'password'], { setCredential });
    type('username', 'ana');
    await submit(true);
    expect(held.calls).toBe(0);
    expect(argv.sent().map(({ name, text }) => [name, text])).toEqual([
      ['username', 'ana'],
      ['password', ''],
    ]);
    expect(heard.sent).toBe(1);
  });

  it('GW-015: a save the hub answers with credentialStored false counts as not saved', async () => {
    const { held, setCredential } = heldSave();
    const { heard } = showPrompt('VNC', ['password'], { setCredential });
    type('password', PASSWORD);
    await submit(true);
    held.answer(false);
    await settle();
    expect(dialog()?.textContent).toContain('did not save the password');
    expect(heard.sent).toBe(0);
    expect(document.activeElement).toBe(button('Close'));
  });
});
