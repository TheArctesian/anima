// @vitest-environment jsdom
import { expect, it, vi } from 'vitest';
import { saved } from '../check.fixture';
import { button, choose, hasInput, input, installFormHarness, show, submit, type } from '../form.fixture';

vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('../../../motion/engine.fixture')).fakeRun,
}));

installFormHarness();

const OK = { ok: true, value: {} } as const;

it('CHK-009: DOMAIN_EXPIRY is offered and shows domain, warn days and timeout with their defaults', () => {
  show();
  choose('Type', 'DOMAIN_EXPIRY');
  for (const label of ['Domain', 'Warn days', 'Timeout (ms)']) expect(hasInput(label)).toBe(true);
  expect((input('Warn days') as HTMLInputElement).value).toBe('30');
  expect((input('Timeout (ms)') as HTMLInputElement).value).toBe('10000');
});

it('D-29: a new domain expiry check proposes 12 hours', () => {
  show();
  choose('Type', 'DOMAIN_EXPIRY');
  expect((input('Interval (seconds)') as HTMLInputElement).value).toBe('43200');
  choose('Type', 'TCP');
  expect((input('Interval (seconds)') as HTMLInputElement).value).toBe('60');
});

it('D-29: a touched interval is kept when the type changes', () => {
  show();
  type('Interval (seconds)', '7200');
  choose('Type', 'DOMAIN_EXPIRY');
  expect((input('Interval (seconds)') as HTMLInputElement).value).toBe('7200');
});

it('D-29: an interval under 1 hour is refused with a fixed sentence and nothing is sent', async () => {
  const onSave = vi.fn(() => Promise.resolve(OK));
  const form = show({ onSave });
  choose('Type', 'DOMAIN_EXPIRY');
  type('Name', 'Registration');
  type('Domain', 'example.net');
  type('Interval (seconds)', '3599');
  await submit(form);
  expect(onSave).not.toHaveBeenCalled();
  expect(document.body.textContent).toContain('A domain expiry check runs at most once an hour: enter 3600 seconds or more.');
});

it('CHK-009: saving sends the domain, warn days and timeout', async () => {
  const onSave = vi.fn(() => Promise.resolve(OK));
  const form = show({ onSave });
  choose('Type', 'DOMAIN_EXPIRY');
  type('Name', 'Registration');
  type('Domain', 'example.net');
  await submit(form);
  expect(onSave).toHaveBeenCalledTimes(1);
  const [values] = onSave.mock.calls[0] as unknown as [{ domainExpiry: unknown; intervalSeconds: string }];
  expect(values.domainExpiry).toMatchObject({ domain: 'example.net', warnDays: '30', timeoutMs: '10000' });
  expect(values.intervalSeconds).toBe('43200');
});

it('CHK-009: an existing domain expiry check loads its own values and shows days left', () => {
  const check = saved('DOMAIN_EXPIRY', { __typename: 'DomainExpiryConfig', domain: 'example.net', warnDays: 14, timeoutMs: 5000 }, { intervalSeconds: 43_200, domainDaysLeft: 9 });
  show({ check });
  expect((input('Domain') as HTMLInputElement).value).toBe('example.net');
  expect((input('Warn days') as HTMLInputElement).value).toBe('14');
  expect((input('Timeout (ms)') as HTMLInputElement).value).toBe('5000');
  expect(document.body.textContent).toContain('Days left: 9 days');
  expect(button('Save check')).toBeDefined();
});
