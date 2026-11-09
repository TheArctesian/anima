// @vitest-environment jsdom
import { expect, it, vi } from 'vitest';
import { choose, hasInput, input, installFormHarness, show } from './form.fixture';

vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('../../motion/engine.fixture')).fakeRun,
}));

installFormHarness();

it('R-102, D-29: the domain expiry form has no retries or retry interval fields', () => {
  show();
  choose('Type', 'DOMAIN_EXPIRY');
  expect(hasInput('Interval (seconds)')).toBe(true);
  expect(hasInput('Retries before Down')).toBe(false);
  expect(hasInput('Retry interval (seconds)')).toBe(false);
});

it('R-102: another type (HTTP) still shows retries and the retry interval with their defaults', () => {
  show();
  choose('Type', 'HTTP');
  expect(input('Retries before Down').value).toBe('3');
  expect(hasInput('Retry interval (seconds)')).toBe(true);
});
