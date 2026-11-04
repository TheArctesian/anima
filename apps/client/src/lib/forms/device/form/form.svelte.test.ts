// @vitest-environment jsdom
import { flushSync } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import type { HubResult } from '$lib/hub/error';
import { installAnimations } from '../../animations.fixture';
import { credentialInput } from '../input';
import type { ConnectionTestResult } from '../hub-types';
import { HOST_KEY, KEY, OK, SAVED, button, choose, input, leave, save, settle, show, type } from './form.fixture';

vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('../../../motion/engine.fixture')).fakeRun,
}));

beforeEach(() => {
  installAnimations();
});

afterEach(leave);


it('DEV-011: the password and the passphrase are hidden text that the browser does not remember', () => {
  show(SAVED);
  expect(input('credential-password').type).toBe('password');
  expect(input('credential-password').autocomplete).toBe('new-password');

  choose('PRIVATE_KEY');

  expect(input('credential-passphrase').type).toBe('password');
  expect(input('credential-passphrase').autocomplete).toBe('new-password');
});

it('DEV-012: Private key shows the key and passphrase, and save sends the privateKey credential', async () => {
  const { onSave } = show(SAVED);
  choose('PRIVATE_KEY');
  expect(input('credential-privateKey-key').tagName).toBe('TEXTAREA');
  expect(input('credential-privateKey-key').getAttribute('spellcheck')).toBe('false');
  expect(document.getElementById('device-credential-password')).toBeNull();

  type('credential-privateKey-key', KEY);
  type('credential-passphrase', 'open sesame');
  await save();

  const [values] = onSave.mock.calls[0] ?? [];
  expect(values && credentialInput(values)).toEqual({ privateKey: { key: KEY, passphrase: 'open sesame' } });
});

it('DEV-010: an invalid host is not saved; the host input is flagged, described and focused', async () => {
  const { onSave } = show(SAVED);
  type('host', 'not a host!');

  await save();

  expect(onSave).not.toHaveBeenCalled();
  expect(input('host').getAttribute('aria-invalid')).toBe('true');
  const describedBy = input('host').getAttribute('aria-describedby') ?? '';
  expect(document.getElementById(describedBy)?.textContent).toContain('valid host');
  expect(document.activeElement).toBe(input('host'));
});

it('DEV-010: the host is checked when the input loses focus', () => {
  show(SAVED);
  type('host', '-bad-');

  input('host').dispatchEvent(new FocusEvent('blur'));
  flushSync();

  expect(input('host').getAttribute('aria-invalid')).toBe('true');
});

it('DEV-010: a hub VALIDATION error for host lands on the host input; another error is a form alert', async () => {
  const rejected: HubResult<unknown> = {
    ok: false,
    error: { code: 'VALIDATION', message: 'Invalid input.', fields: [{ field: 'host', message: 'The hub does not accept this host.' }] },
  };
  const { onSave } = show(SAVED);
  onSave.mockResolvedValueOnce(rejected);

  await save();

  expect(document.getElementById('device-host-error')?.textContent).toBe('The hub does not accept this host.');
  expect(input('host').getAttribute('aria-invalid')).toBe('true');

  onSave.mockResolvedValueOnce({ ok: false, error: { code: 'CONFLICT', message: 'Someone else changed this device.' } });
  await save();
  expect(document.querySelector('form [role="alert"]')?.textContent).toContain('Someone else changed this device.');
});

it('D-4: the secret inputs are empty after a successful save, and kept after a failed one', async () => {
  const { onSave } = show(SAVED);
  type('credential-password', 'hunter2');
  onSave.mockResolvedValueOnce({ ok: false, error: { code: 'UNAVAILABLE', message: 'Try again.' } });

  await save();
  expect(input('credential-password').value).toBe('hunter2');

  await save();
  expect(onSave).toHaveBeenCalledTimes(2);
  expect(input('credential-password').value).toBe('');
});

it('DEV-013: Test sends the unsaved shape (no device ID) once the host changed, and says the stored credential is not used', async () => {
  const { onTest } = show(SAVED);
  expect(document.body.textContent).not.toContain('does not use the stored credential');
  type('host', 'nas2.home.arpa');

  button('Test connection').click();
  await settle();

  expect(onTest).toHaveBeenCalledExactlyOnceWith({
    host: 'nas2.home.arpa',
    ssh: { enabled: true, port: 22, username: 'root', auth: 'PASSWORD' },
  });
  expect(document.body.textContent).toContain('does not use the stored credential');
});

it('DEV-013: Test of an unchanged saved device sends its ID', async () => {
  const { onTest } = show(SAVED);

  button('Test connection').click();
  await settle();

  expect(onTest).toHaveBeenCalledExactlyOnceWith({ deviceId: 'd1' });
  expect(document.body.textContent).toContain('accepted the credential');
});

it('DEV-013: a refused test is not run on an invalid host', async () => {
  const { onTest } = show(SAVED);
  type('host', '');

  button('Test connection').click();
  await settle();

  expect(onTest).not.toHaveBeenCalled();
  expect(input('host').getAttribute('aria-invalid')).toBe('true');
});

const OUTCOMES: readonly [string, ConnectionTestResult, string][] = [
  ['unreachable', { reachable: false, authenticated: false, hostKeyChanged: false, message: 'Connection timed out.' }, 'Connection timed out.'],
  ['authentication failed', { reachable: true, authenticated: false, hostKeyChanged: false }, 'The device refused the credential.'],
  [
    'changed key without a key',
    { reachable: true, authenticated: false, hostKeyChanged: true },
    'The host key changed. Do not trust this device until you check it.',
  ],
];

it.each(OUTCOMES)('DEV-013, SEC-007: the %s outcome shows its message', async (_name, result, text) => {
  show(SAVED, {}, { ok: true, value: result });

  button('Test connection').click();
  await settle();

  expect(document.querySelector('.test [role="status"]')?.textContent).toContain(text);
  expect(document.querySelector('[role="dialog"]')).toBeNull();
});

it('DEV-013: a hub error from the test is shown as an alert', async () => {
  show(SAVED, {}, { ok: false, error: { code: 'unreachable', message: 'The hub did not answer.' } });

  button('Test connection').click();
  await settle();

  expect(document.querySelector('.test [role="alert"]')?.textContent).toBe('The hub did not answer.');
});

it('DEV-013, SEC-006: a new host key opens the host key modal; Accept on a saved device sends the fingerprint', async () => {
  const { onAcceptHostKey } = show(SAVED, {}, {
    ok: true,
    value: { reachable: true, authenticated: false, hostKeyChanged: false, hostKey: HOST_KEY },
  });

  button('Test connection').click();
  await settle();

  expect(document.querySelector('[role="dialog"]')?.textContent).toContain('SHA256:abc');
  button('Accept').click();
  await settle();
  expect(onAcceptHostKey).toHaveBeenCalledExactlyOnceWith('SHA256:abc');
  expect(document.querySelector('[role="dialog"]')).toBeNull();
});

it('SEC-007: a changed host key opens the modal as a warning', async () => {
  show(SAVED, {}, {
    ok: true,
    value: { reachable: true, authenticated: true, hostKeyChanged: true, hostKey: HOST_KEY },
  });

  button('Test connection').click();
  await settle();

  expect(document.querySelector('[role="dialog"] [role="alert"]')?.textContent).toContain('Do not trust this device');
});

it('DEV-022: the RDP section is text only', () => {
  show(SAVED, { section: 'rdp' });

  const section = document.querySelector('[data-section="rdp"]');
  expect(section?.textContent).toContain('RDP comes in a later version of Anima.');
  expect(section?.querySelectorAll('input, select, textarea')).toHaveLength(0);
  expect(document.activeElement?.closest('[data-section="rdp"]')).not.toBeNull();
});

it('DEV-010: tags are added with Enter or comma, normalised, and removed with Backspace', () => {
  show(null);
  const tags = input('tags');
  const press = (key: string): void => {
    tags.dispatchEvent(new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true }));
    flushSync();
  };
  const chips = (): string[] => [...document.querySelectorAll('.chip')].map((chip) => (chip.firstChild?.textContent ?? '').trim());

  tags.value = ' lab ';
  tags.dispatchEvent(new Event('input', { bubbles: true }));
  press('Enter');
  tags.value = 'lab';
  tags.dispatchEvent(new Event('input', { bubbles: true }));
  press(',');
  tags.value = 'edge';
  tags.dispatchEvent(new Event('input', { bubbles: true }));
  press(',');
  expect(chips()).toEqual(['lab', 'edge']);

  press('Backspace');
  expect(chips()).toEqual(['lab']);
});

it('D-6: the site picker shows only in create mode with more than one site, and the chosen site is sent', async () => {
  show(SAVED, { sites: [{ id: 's1', name: 'Home' }, { id: 's2', name: 'Office' }] });
  expect(document.getElementById('device-siteId')).toBeNull();
  leave();
  document.body.replaceChildren();

  const { onSave } = show(null, { sites: [{ id: 's1', name: 'Home' }, { id: 's2', name: 'Office' }] });
  const picker = document.getElementById('device-siteId') as HTMLSelectElement;
  picker.value = 's2';
  picker.dispatchEvent(new Event('change', { bubbles: true }));
  type('name', 'nas');
  type('host', 'nas.home.arpa');
  await save();

  expect(onSave.mock.calls[0]?.[1]).toBe('s2');
});

it('D-6: with one site there is no picker and that site is used', async () => {
  const { onSave } = show(null, { sites: [{ id: 's1', name: 'Home' }] });
  type('name', 'nas');
  type('host', 'nas.home.arpa');

  await save();

  expect(document.getElementById('device-siteId')).toBeNull();
  expect(onSave.mock.calls[0]?.[1]).toBe('s1');
});
