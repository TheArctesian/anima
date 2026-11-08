// @vitest-environment jsdom
import { flushSync } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import type { HubResult } from '$lib/hub/hub';
import { installAnimations } from '../../../forms/animations.fixture';
import type { ConnectionTestResult } from '../connection-test';
import { testResult } from '../result.fixture';
import { HOST_KEY, SAVED, button, input, leave, save, settle, show, type } from './form.fixture';

vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('../../../motion/engine.fixture')).fakeRun,
}));

beforeEach(() => {
  installAnimations();
});

afterEach(leave);

const NEW_KEY: HubResult<ConnectionTestResult> = {
  ok: true,
  value: testResult({ reachable: true, authenticated: false, hostKeyChanged: false, hostKey: HOST_KEY }),
};

async function test(): Promise<void> {
  button('Test connection').click();
  await settle();
}

const status = (): string => document.querySelector('.test [role="status"]')?.textContent.trim() ?? '';

it('SEC-007, D-5: after the host was edited, a new key cannot be accepted until the device is saved and tested again', async () => {
  show(SAVED, {}, NEW_KEY);
  type('host', 'other.home.arpa');

  await test();

  const dialog = document.querySelector('[role="dialog"]');
  expect(dialog?.textContent).toContain('SHA256:abc');
  expect(dialog?.textContent).toContain('Save the device, then test again to accept this key.');
  expect(() => button('Accept')).toThrow();
});

it('DEV-013: Test connection works with an empty name, since only the host and SSH settings are tested', async () => {
  const { onTest } = show(SAVED);
  type('name', '');

  await test();

  expect(onTest).toHaveBeenCalledOnce();
});

it('DEV-013: the result is dropped when the tested host or SSH settings change, and so is an error', async () => {
  const { onTest } = show(SAVED);
  await test();
  expect(status()).toContain('accepted the credential');

  type('host', 'other.home.arpa');
  expect(status()).toBe('');

  onTest.mockResolvedValueOnce({ ok: false, error: { code: 'unreachable', message: 'The hub did not answer.' } });
  await test();
  expect(document.querySelector('.test [role="alert"]')).not.toBeNull();
  type('ssh-port', '2200');
  expect(document.querySelector('.test [role="alert"]')).toBeNull();
});

it('DEV-013: a change to the name keeps the result of the test', async () => {
  show(SAVED);
  await test();

  type('name', 'nas2');

  expect(status()).toContain('accepted the credential');
});

it('SEC-007: a changed key whose fingerprint is unknown is styled as a warning; a good result is not', async () => {
  show(SAVED, {}, { ok: true, value: testResult({ reachable: true, authenticated: true, hostKeyChanged: true }) });
  await test();
  expect(document.querySelector('.test [role="status"] p')?.classList.contains('warning')).toBe(true);
  leave();

  show(SAVED);
  await test();
  expect(document.querySelector('.test [role="status"] p')?.classList.contains('warning')).toBe(false);
});

it('SEC-006: after Accept succeeds the result and the dialog are gone', async () => {
  show(SAVED, {}, NEW_KEY);
  await test();

  button('Accept').click();
  await settle();

  expect(status()).toBe('');
  expect(document.querySelector('[role="dialog"]')).toBeNull();
});

it('DEV-010: an invalid SSH field opens a collapsed SSH section and gets the focus', async () => {
  const { onSave } = show(SAVED);
  type('ssh-port', 'abc');
  const trigger = document.querySelector<HTMLButtonElement>('[data-section="ssh"] h3 button');
  trigger?.click();
  flushSync();
  expect(trigger?.getAttribute('aria-expanded')).toBe('false');

  await save();

  expect(onSave).not.toHaveBeenCalled();
  expect(trigger?.getAttribute('aria-expanded')).toBe('true');
  expect(document.activeElement).toBe(input('ssh-port'));
});

it('D-6: editing sends no site, even when sites are given', async () => {
  const { onSave } = show(SAVED, { sites: [{ id: 's1', name: 'Home', subnet: '10.10.0.0/24' }, { id: 's2', name: 'Office', subnet: '10.20.0.0/24' }] });

  await save();

  expect(onSave).toHaveBeenCalledOnce();
  expect(onSave.mock.calls[0]?.[1]).toBeNull();
});

it('D-2: two form-level errors with the same text are both shown', async () => {
  const { onSave } = show(SAVED);
  const same = 'The hub cannot use this.';
  onSave.mockResolvedValueOnce({
    ok: false,
    error: { code: 'VALIDATION', message: 'Invalid.', fields: [{ field: 'kind', message: same }, { field: 'os', message: same }] },
  });

  await save();

  expect([...document.querySelectorAll('form > .alert p')].map((alert) => alert.textContent)).toEqual([same, same]);
});

it('DEV-010: a tag is added with Enter alone, and the change is announced politely', () => {
  show(null);
  const tags = input('tags');
  tags.value = 'edge';
  tags.dispatchEvent(new Event('input', { bubbles: true }));

  tags.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true, cancelable: true }));
  flushSync();

  expect([...document.querySelectorAll('.chip')].map((chip) => chip.firstChild?.textContent?.trim())).toEqual(['edge']);
  const live = document.querySelector('.announcement');
  expect(live?.getAttribute('aria-live')).toBe('polite');
  expect(live?.textContent).toBe('Added tag edge');

  document.querySelector<HTMLButtonElement>('.chip button')?.click();
  flushSync();
  expect(live?.textContent).toBe('Removed tag edge');
  expect(document.querySelectorAll('.chip')).toHaveLength(0);
});

it('DEV-013: Test connection clicked twice while the first test is pending calls onTest once', async () => {
  const { onTest } = show(SAVED);
  onTest.mockImplementation(() => new Promise(() => undefined));

  button('Test connection').click();
  button('Test connection').click();
  await settle();

  expect(onTest).toHaveBeenCalledOnce();
});

it('DEV-010: saving twice while the first save is pending (click, then a form submit) calls onSave once', async () => {
  const { onSave } = show(SAVED);
  onSave.mockImplementation(() => new Promise(() => undefined));

  button('Save').click();
  document.querySelector('form')?.requestSubmit();
  await settle();

  expect(onSave).toHaveBeenCalledOnce();
});

it('D-4: Remove credential clicked twice while the first call is pending calls onClearCredential once', async () => {
  const onClearCredential = vi.fn(() => new Promise<HubResult<unknown>>(() => undefined));
  show(SAVED, { credentialStored: true, onClearCredential });

  button('Remove credential').click();
  button('Remove credential').click();
  await settle();

  expect(onClearCredential).toHaveBeenCalledOnce();
});

it('D-4: a test result does not outlive a successful save', async () => {
  show(SAVED);
  await test();
  expect(status()).toContain('accepted the credential');

  await save();

  expect(status()).toBe('');
});

it('Settings tab: outside a modal, Cancel and Save stay at the end of the form, after the sections', () => {
  show(SAVED);

  const form = document.querySelector('form');
  expect(form?.lastElementChild?.contains(button('Save'))).toBe(true);
  expect(form?.contains(button('Cancel'))).toBe(true);
});
