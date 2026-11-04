// @vitest-environment jsdom
import { flushSync, mount, tick, unmount } from 'svelte';
import { afterEach, beforeAll, expect, it, vi } from 'vitest';
import type { HubResult } from '$lib/hub/error';
import CheckForm from './CheckForm.svelte';
import type { SiblingCheck } from './model/group';
import type { CheckFields } from './model/hub-types';
import type { CheckFormValues } from './model/values';

vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('../../motion/engine.fixture')).fakeRun,
}));

const OK: HubResult<unknown> = { ok: true, value: {} };
let app: ReturnType<typeof mount> | undefined;

// jsdom has no Web Animations; Svelte's transitions need an animation that never finishes on its own.
beforeAll(() => {
  Element.prototype.animate = () =>
    ({ onfinish: null, cancel: () => undefined, finish: () => undefined, currentTime: 0 }) as unknown as Animation;
});

afterEach(() => {
  if (app !== undefined) void unmount(app);
  app = undefined;
  document.body.innerHTML = '';
});

interface Options {
  check?: CheckFields | null;
  siblings?: SiblingCheck[];
  onSave?: (values: CheckFormValues) => Promise<HubResult<unknown>>;
}

function show({ check = null, siblings = [], onSave = () => Promise.resolve(OK) }: Options = {}): HTMLFormElement {
  app = mount(CheckForm, { target: document.body, props: { check, siblings, onSave, onCancel: () => undefined } });
  flushSync();
  return document.querySelector('form') as HTMLFormElement;
}

function input(label: string): HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement {
  const found = [...document.querySelectorAll('label')].find((candidate) => candidate.textContent.trim() === label);
  const target = found?.control ?? null;
  if (target === null) throw new Error(`no input labelled ${label}`);
  return target as HTMLInputElement;
}

function type(label: string, value: string): void {
  const field = input(label);
  field.value = value;
  field.dispatchEvent(new Event('input', { bubbles: true }));
  flushSync();
}

function choose(label: string, value: string): void {
  const field = input(label);
  field.value = value;
  field.dispatchEvent(new Event('change', { bubbles: true }));
  flushSync();
}

async function submit(form: HTMLFormElement): Promise<void> {
  form.dispatchEvent(new SubmitEvent('submit', { bubbles: true, cancelable: true }));
  await tick();
  await Promise.resolve();
  flushSync();
}

const HTTP_SAVED: CheckFields = {
  id: 'chk-1',
  type: 'HTTP',
  name: 'site',
  intervalSeconds: 60,
  retries: 3,
  retryIntervalSeconds: 20,
  degradedThresholdMs: 1000,
  invert: false,
  config: {
    __typename: 'HttpConfig',
    url: 'https://example.com',
    method: 'GET',
    headers: [{ name: 'Authorization', value: null }],
    body: null,
    maxRedirects: 10,
    acceptedStatus: [{ from: 200, to: 299 }],
    keyword: null,
    jsonQuery: null,
    jsonExpected: null,
    authKind: 'BEARER',
    verifyTls: true,
    timeoutMs: 10000,
  },
};

it('CHK-009: the form refuses to save an invalid check and shows the error beside its input', async () => {
  const onSave = vi.fn(() => Promise.resolve(OK));
  const form = show({ onSave });
  type('Name', 'ssh');
  choose('Type', 'TCP');
  type('Port', '70000');

  await submit(form);

  expect(onSave).not.toHaveBeenCalled();
  const port = input('Port');
  expect(port.getAttribute('aria-invalid')).toBe('true');
  const message = document.getElementById(port.getAttribute('aria-describedby') ?? '');
  expect(message?.textContent).toBe('Enter a port from 1 to 65535.');
});

it('CHK-009: a valid form saves the typed values', async () => {
  const onSave = vi.fn((_values: CheckFormValues) => Promise.resolve(OK));
  const form = show({ onSave });
  type('Name', 'ssh');
  choose('Type', 'TCP');
  type('Port', '22');

  await submit(form);

  expect(onSave).toHaveBeenCalledOnce();
  expect(onSave.mock.calls[0]?.[0]).toMatchObject({ type: 'TCP', name: 'ssh', tcp: { port: '22', timeoutMs: '5000' } });
});

it('CHK-009: a hub VALIDATION error shows on its field', async () => {
  const onSave = vi.fn(() =>
    Promise.resolve<HubResult<unknown>>({
      ok: false,
      error: { code: 'VALIDATION', message: 'Invalid', fields: [{ field: 'timeoutMs', message: 'This value must be between 100 and 60000.' }] },
    }),
  );
  const form = show({ onSave });
  type('Name', 'ping');

  await submit(form);

  const timeout = input('Timeout (ms)');
  expect(timeout.getAttribute('aria-invalid')).toBe('true');
  expect(document.getElementById(timeout.getAttribute('aria-describedby') ?? '')?.textContent).toBe(
    'This value must be between 100 and 60000.',
  );
});

it('CHK-009: a hub error with no input is listed at the top', async () => {
  const form = show({
    onSave: () => Promise.resolve({ ok: false, error: { code: 'CONFLICT', message: 'Name already used.' } }),
  });
  type('Name', 'ping');

  await submit(form);

  expect(document.querySelector('[role="alert"]')?.textContent).toContain('Name already used.');
});

it('CHK-009: in edit mode the type is fixed: there is no type picker', () => {
  show({ check: HTTP_SAVED });
  expect([...document.querySelectorAll('label')].some((label) => label.textContent.trim() === 'Type')).toBe(false);
  expect(document.body.textContent).toContain('Type: HTTP or HTTPS');
});

it('CHK-009: secret inputs are password fields and are never prefilled', () => {
  show({ check: HTTP_SAVED });
  const header = input('Header value') as HTMLInputElement;
  expect(header.type).toBe('password');
  expect(header.value).toBe('');
  expect(input('Bearer token')).toMatchObject({ type: 'password', value: '' });
});

it('CHK-009: a new basic-auth password is a password field', () => {
  show();
  choose('Type', 'HTTP');
  choose('Authentication', 'BASIC');
  expect((input('Password') as HTMLInputElement).type).toBe('password');
  expect((input('User name') as HTMLInputElement).type).toBe('text');
});

it('CHK-008: invert sits in the advanced settings, which open on request with aria-expanded', async () => {
  show();
  const toggle = [...document.querySelectorAll('button')].find((button) => button.textContent.trim() === 'Advanced settings');
  expect(toggle?.getAttribute('aria-expanded')).toBe('false');
  expect(document.body.textContent).not.toContain('Invert');

  toggle?.click();
  await tick();
  flushSync();

  expect(toggle?.getAttribute('aria-expanded')).toBe('true');
  expect((input('Invert') as HTMLInputElement).checked).toBe(false);
  expect((input('Interval (seconds)') as HTMLInputElement).value).toBe('60');
});

it('CHK-009: an error in an advanced setting opens the advanced settings', async () => {
  const onSave = vi.fn(() => Promise.resolve(OK));
  const form = show({ onSave });
  type('Name', 'ping');
  const toggle = [...document.querySelectorAll('button')].find((button) => button.textContent.trim() === 'Advanced settings');
  toggle?.click();
  await tick();
  flushSync();
  type('Interval (seconds)', '0');
  toggle?.click();
  await tick();
  flushSync();

  await submit(form);

  expect(input('Interval (seconds)').getAttribute('aria-invalid')).toBe('true');
});

it('Section 6.4: a group offers only the other checks that cannot make a cycle', () => {
  const siblings: SiblingCheck[] = [
    { id: 'a', name: 'ping', type: 'ICMP', childIds: [] },
    { id: 'self', name: 'self-group', type: 'GROUP', childIds: [] },
    { id: 'outer', name: 'outer-group', type: 'GROUP', childIds: ['self'] },
  ];
  show({
    check: { ...HTTP_SAVED, id: 'self', type: 'GROUP', name: 'self-group', config: { __typename: 'GroupConfig', children: [] } },
    siblings,
  });
  const names = [...document.querySelectorAll('fieldset label')].map((label) => label.textContent.trim());
  expect(names).toEqual(['ping']);
});
