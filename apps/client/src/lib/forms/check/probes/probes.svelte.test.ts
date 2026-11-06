// @vitest-environment jsdom
import { expect, it, vi } from 'vitest';
import { saved } from '../check.fixture';
import { choose, hasInput, input, installFormHarness, messageOf, messageOfHint, show, submit, type } from '../form.fixture';

vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('../../../motion/engine.fixture')).fakeRun,
}));

installFormHarness();

it('CHK-009: the type list offers the four Phase 2 types in operator words', () => {
  show();
  const labels = [...(input('Type') as HTMLSelectElement).options].map((option) => option.textContent);
  expect(labels).toEqual(expect.arrayContaining(['Docker container', 'systemd service', 'SNMP', 'Tailscale ping']));
});

it.each([
  ['DOCKER', ['Container name', 'Timeout (ms)']],
  ['SYSTEMD', ['Unit name', 'Timeout (ms)']],
  ['TAILSCALE_PING', ['Target', 'Timeout (ms)']],
  ['SNMP', ['Port', 'Version', 'OID', 'Expected value', 'Community', 'Timeout (ms)']],
] as const)('CHK-009: choosing %s shows its fields', (type_, labels) => {
  show();
  choose('Type', type_);
  for (const label of labels) expect(hasInput(label)).toBe(true);
});

it('CHK-009: SNMP v3 shows the user name and the two passwords instead of the community', () => {
  show();
  choose('Type', 'SNMP');
  choose('Version', 'V3');
  for (const label of ['User name', 'Authentication password', 'Privacy password']) expect(hasInput(label)).toBe(true);
  expect(hasInput('Community')).toBe(false);
  expect(messageOfHint('User name')).toContain('authPriv, SHA-256 and AES-128');
});

it('DEV-011: the SNMP community and v3 passwords are hidden text', () => {
  show();
  choose('Type', 'SNMP');
  expect(input('Community').getAttribute('type')).toBe('password');
  choose('Version', 'V3');
  expect(input('Authentication password').getAttribute('type')).toBe('password');
  expect(input('Privacy password').getAttribute('type')).toBe('password');
});

it('CHK-012: a container name that looks like shell is refused beside its input and not saved', async () => {
  const onSave = vi.fn(() => Promise.resolve({ ok: true as const, value: {} }));
  const form = show({ onSave });
  type('Name', 'web');
  choose('Type', 'DOCKER');
  type('Container name', 'web; id');
  await submit(form);
  expect(onSave).not.toHaveBeenCalled();
  expect(messageOf('Container name')).toContain('letters, digits');
});

it('SEC-005: editing an SNMP check leaves the secrets empty and says that empty keeps them', () => {
  show({
    check: saved('SNMP', {
      __typename: 'SnmpConfig',
      port: 161,
      version: 'V2C',
      oid: '1.3.6.1.2.1.1.3.0',
      expected: null,
      username: null,
      timeoutMs: 5000,
    }),
  });
  expect((input('Community') as HTMLInputElement).value).toBe('');
  expect(messageOfHint('Community')).toContain('keep the stored value');
});
