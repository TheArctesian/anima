// @vitest-environment jsdom
import { expect, it, vi } from 'vitest';
import type { FieldError, HubResult } from '$lib/hub/hub';
import { BEARER_SAVED, HTTP_SAVED } from './check.fixture';
import { button, choose, installFormHarness, input, messageOf, show, submit, type } from './form.fixture';
import type { CheckFields } from './model/hub-types';
import type { CheckFormValues } from './model/values';

vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('../../motion/engine.fixture')).fakeRun,
}));

installFormHarness();

function refuse(...fields: FieldError[]): (values: CheckFormValues) => Promise<HubResult<unknown>> {
  return () => Promise.resolve({ ok: false, error: { code: 'VALIDATION', message: 'Invalid', fields } });
}

it.each([
  ['intervalSeconds', 'Interval (seconds)'],
  ['retries', 'Retries before Down'],
  ['retryIntervalSeconds', 'Retry interval (seconds)'],
  ['degradedThresholdMs', 'Degraded threshold (ms)'],
])('D-2: a hub error on %s opens Advanced and shows beside its input', async (field, label) => {
  const form = show({ onSave: refuse({ field, message: 'The hub refuses this.' }) });
  type('Name', 'ping');
  expect(button('Advanced settings').getAttribute('aria-expanded')).toBe('false');

  await submit(form);

  expect(button('Advanced settings').getAttribute('aria-expanded')).toBe('true');
  expect(input(label).getAttribute('aria-invalid')).toBe('true');
  expect(messageOf(label)).toBe('The hub refuses this.');
});

it('D-2: a hub error for a field the form has no input for is listed at the top', async () => {
  const form = show({ onSave: refuse({ field: 'parentId', message: 'The parent must be a group.' }) });
  type('Name', 'ping');

  await submit(form);

  expect(document.querySelector('[role="alert"]')?.textContent).toContain('The parent must be a group.');
});

it('D-2: a hub error on childIds shows in the group', async () => {
  const form = show({ onSave: refuse({ field: 'childIds', message: 'A group takes at most 100 children.' }) });
  type('Name', 'g');
  choose('Type', 'GROUP');

  await submit(form);

  expect(document.querySelector('fieldset')?.textContent).toContain('A group takes at most 100 children.');
});

const DNS_SAVED: CheckFields = { ...BEARER_SAVED, type: 'DNS', config: { __typename: 'DnsConfig', name: 'a.example', recordType: 'A', resolver: null, expected: null } };

it('D-2: the hub name error about the config goes to the DNS name input', async () => {
  const form = show({ check: DNS_SAVED, onSave: refuse({ field: 'name', message: 'This value is not valid.' }) });
  type('Name to look up', 'b.example');

  await submit(form);

  expect(messageOf('Name to look up')).toBe('This value is not valid.');
  expect(input('Name').getAttribute('aria-invalid')).toBeNull();
});

it('D-2: the hub name error about the check name stays on the Name input, also for DNS', async () => {
  const form = show({ check: DNS_SAVED, onSave: refuse({ field: 'name', message: 'Another check on this device has that name.' }) });
  type('Name', 'taken');

  await submit(form);

  expect(messageOf('Name')).toBe('Another check on this device has that name.');
  expect(input('Name to look up').getAttribute('aria-invalid')).toBeNull();
});

it('D-2: the hub auth error goes to the bearer token input', async () => {
  const form = show({ check: BEARER_SAVED, onSave: refuse({ field: 'auth', message: 'Enter the password or token.' }) });
  type('URL', 'https://example.com/x');

  await submit(form);

  expect(messageOf('Bearer token')).toBe('Enter the password or token.');
});

it('D-2: the hub auth error goes to the password input when it asks for the password, else to the user name', async () => {
  const password = show({ check: HTTP_SAVED, onSave: refuse({ field: 'auth', message: 'Enter the password or token.' }) });
  type('User name', 'probe');
  type('Timeout (ms)', '5000');
  await submit(password);
  expect(messageOf('Password')).toBe('Enter the password or token.');
  expect(input('User name').getAttribute('aria-invalid')).toBeNull();
});

it('D-2: the hub auth error that is not about the password goes to the user name', async () => {
  const form = show({ check: HTTP_SAVED, onSave: refuse({ field: 'auth', message: 'This value is not valid.' }) });
  type('User name', 'probe');
  type('Timeout (ms)', '5000');

  await submit(form);

  expect(messageOf('User name')).toBe('This value is not valid.');
});
