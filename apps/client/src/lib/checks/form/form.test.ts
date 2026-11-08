// @vitest-environment jsdom
import { flushSync } from 'svelte';
import { expect, it, vi } from 'vitest';
import type { HubResult } from '$lib/hub/hub';
import { BEARER_SAVED, HTTP_SAVED, saved } from './check.fixture';
import { button, choose, hasInput, input, installFormHarness, messageOf, messageOfHint, OK, settle, show, submit, type } from './form.fixture';
import type { SiblingCheck } from './model/group';
import type { CheckFormValues } from './model/values';

vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('../../motion/engine.fixture')).fakeRun,
}));

installFormHarness();

type Save = (values: CheckFormValues) => Promise<HubResult<unknown>>;
const saver = () => vi.fn<Save>(() => Promise.resolve(OK));

it('CHK-009: the form refuses to save an invalid check and shows the error beside its input', async () => {
  const onSave = saver();
  const form = show({ onSave });
  type('Name', 'ssh');
  choose('Type', 'TCP');
  type('Port', '70000');

  await submit(form);

  expect(onSave).not.toHaveBeenCalled();
  expect(input('Port').getAttribute('aria-invalid')).toBe('true');
  expect(messageOf('Port')).toBe('Enter a port from 1 to 65535.');
});

it('CHK-009: a valid form saves the typed values', async () => {
  const onSave = saver();
  const form = show({ onSave });
  type('Name', 'ssh');
  choose('Type', 'TCP');
  type('Port', '22');

  await submit(form);

  expect(onSave).toHaveBeenCalledOnce();
  expect(onSave.mock.calls[0]?.[0]).toMatchObject({ type: 'TCP', name: 'ssh', tcp: { port: '22', timeoutMs: '5000' } });
});

it('CHK-009: a second submit while the first is saving does nothing', async () => {
  let finish: (result: HubResult<unknown>) => void = () => undefined;
  const onSave = vi.fn<Save>(() => new Promise((done) => (finish = done)));
  const form = show({ onSave });
  type('Name', 'ping');

  await submit(form);
  await submit(form);
  expect(onSave).toHaveBeenCalledOnce();
  expect(button('Add check').disabled).toBe(true);

  finish(OK);
  await settle();
  expect(button('Add check').disabled).toBe(false);
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
  show({ check: BEARER_SAVED });
  expect(hasInput('Type')).toBe(false);
  expect(document.body.textContent).toContain('Type: HTTP or HTTPS');
});

it('CHK-009: edit mode validates against the saved values: a rename of a basic-auth check needs no secret', async () => {
  const onSave = saver();
  const form = show({ check: HTTP_SAVED, onSave });
  type('Name', 'renamed');

  await submit(form);

  expect(onSave).toHaveBeenCalledOnce();
  expect(document.querySelector('[aria-invalid="true"]')).toBeNull();
});

it('R-048: a stored basic-auth check says the password must be typed again when the user name changes', () => {
  show({ check: HTTP_SAVED });
  expect(messageOfHint('User name')).toContain('type the password again');
});

it('R-048: a new basic-auth check shows no such hint', () => {
  show();
  choose('Type', 'HTTP');
  choose('Authentication', 'BASIC');
  expect(messageOfHint('User name')).toBe('');
});

it('CHK-009: secret inputs are password fields and are never prefilled', () => {
  show({ check: BEARER_SAVED });
  expect(input('Header value 1')).toMatchObject({ type: 'password', value: '' });
  expect(input('Bearer token')).toMatchObject({ type: 'password', value: '' });
});

it('CHK-009: a new basic-auth password is a password field', () => {
  show();
  choose('Type', 'HTTP');
  choose('Authentication', 'BASIC');
  expect((input('Password') as HTMLInputElement).type).toBe('password');
  expect((input('User name') as HTMLInputElement).type).toBe('text');
});

it('CHK-009: a body over 64 KiB marks the multi-line input invalid', async () => {
  const form = show();
  type('Name', 'site');
  choose('Type', 'HTTP');
  type('URL', 'https://example.com');
  type('Request body', 'x'.repeat(65_537));

  await submit(form);

  expect(input('Request body').getAttribute('aria-invalid')).toBe('true');
  expect(messageOf('Request body')).toContain('65536');
});

it('CHK-009: a warning is shown when saving would lose a stored secret', () => {
  show({ check: BEARER_SAVED });
  type('URL', 'https://other.example');
  flushSync();

  expect(document.querySelector('[role="status"]')?.textContent).toContain('will not reuse the stored secrets');
});

it('Section 6.4: a group offers only the other checks that cannot make a cycle', () => {
  const siblings: SiblingCheck[] = [
    { id: 'a', name: 'ping', type: 'ICMP', childIds: [] },
    { id: 'self', name: 'self-group', type: 'GROUP', childIds: [] },
    { id: 'outer', name: 'outer-group', type: 'GROUP', childIds: ['self'] },
  ];
  show({ check: saved('GROUP', { __typename: 'GroupConfig', children: [] }, { id: 'self', name: 'self-group' }), siblings });
  const names = [...document.querySelectorAll('fieldset label')].map((label) => label.textContent.trim());
  expect(names).toEqual(['ping']);
});

it('Section 6.4: ticking a check puts its id in the group', async () => {
  const onSave = saver();
  const siblings: SiblingCheck[] = [
    { id: 'a', name: 'ping', type: 'ICMP', childIds: [] },
    { id: 'b', name: 'web', type: 'HTTP', childIds: [] },
  ];
  const form = show({ siblings, onSave });
  type('Name', 'both');
  choose('Type', 'GROUP');
  input('web').click();
  flushSync();

  await submit(form);

  expect(onSave.mock.calls[0]?.[0].group.childIds).toEqual(['b']);
});

it('D-2: an invalid Advanced setting found before saving opens Advanced and shows beside its input', async () => {
  const onSave = saver();
  const form = show({ onSave });
  type('Name', 'ping');
  type('Interval (seconds)', '5');
  expect(button('Advanced settings').getAttribute('aria-expanded')).toBe('false');

  await submit(form);

  expect(onSave).not.toHaveBeenCalled();
  expect(button('Advanced settings').getAttribute('aria-expanded')).toBe('true');
  expect(input('Interval (seconds)').getAttribute('aria-invalid')).toBe('true');
});

it('outside a modal, Cancel and the submit button stay at the end of the form', () => {
  const form = show();

  expect(form.lastElementChild?.contains(button('Add check'))).toBe(true);
  expect(form.contains(button('Cancel'))).toBe(true);
});
