import { describe, expect, it } from 'vitest';
import type { HubError } from '$lib/hub/error';
import { connectionTestInput, testOutcome, testsSavedRecord, testsStoredCredential } from './connection-test';
import { FORM_FIELD, fieldErrorsFrom } from './field-errors';
import { testResult } from './result.fixture';
import { createInput, credentialInput, updateInput } from './input';
import { defaultRemoteValues } from './remote/values';
import { validateDevice } from './validate';
import { formValues, type DeviceFields, type DeviceFormValues, type HostKey } from './values';

const SAVED: DeviceFields = {
  name: 'nas',
  host: 'nas.home.arpa',
  kind: 'server',
  os: 'LINUX',
  tags: ['lab', 'storage'],
  ssh: { enabled: true, port: 2222, username: 'root', auth: 'PASSWORD' },
};

function withValues(base: DeviceFormValues, change: Partial<DeviceFormValues>): DeviceFormValues {
  return { ...base, ...change };
}

function withCredential(base: DeviceFormValues, change: Partial<DeviceFormValues['credential']>): DeviceFormValues {
  return { ...base, credential: { ...base.credential, ...change } };
}

function fieldsOf(values: DeviceFormValues): string[] {
  return validateDevice(values).map((error) => error.field);
}

const KEY = '-----BEGIN OPENSSH PRIVATE KEY-----\nabc\n-----END OPENSSH PRIVATE KEY-----\n';
const HOST_KEY: HostKey = { algorithm: 'ssh-ed25519', fingerprintSha256: 'SHA256:abc', firstSeenAt: '2026-11-04T09:00:00+00:00' };

describe('formValues', () => {
  it('DEV-010: a new device starts with the Section 6.2 defaults and empty credentials', () => {
    expect(formValues(null)).toEqual({
      name: '',
      host: '',
      kind: 'server',
      os: 'OTHER',
      tags: [],
      ssh: { enabled: false, port: '22', username: '', auth: 'PASSWORD' },
      credential: { password: '', privateKey: '', passphrase: '' },
      remoteSecrets: { rdpPassword: '', vncPassword: '' },
      ...defaultRemoteValues(),
    });
  });

  it('DEV-010: an edited device shows its saved values and no secret', () => {
    const values = formValues(SAVED);
    expect(values).toMatchObject({ name: 'nas', host: 'nas.home.arpa', os: 'LINUX', tags: ['lab', 'storage'] });
    expect(values.ssh).toEqual({ enabled: true, port: '2222', username: 'root', auth: 'PASSWORD' });
    expect(values.credential).toEqual({ password: '', privateKey: '', passphrase: '' });
  });
});

describe('validateDevice', () => {
  const valid = withValues(formValues(null), { name: 'nas', host: '10.0.0.5' });

  it('DEV-010: a name and a host make a valid form with SSH off', () => {
    expect(validateDevice(valid)).toEqual([]);
  });

  it('DEV-010: a blank name and a blank host are reported on their fields', () => {
    expect(fieldsOf(formValues(null))).toEqual(['name', 'host']);
    expect(fieldsOf(withValues(valid, { name: '   ' }))).toEqual(['name']);
  });

  it('DEV-010: an invalid host is reported on host', () => {
    expect(fieldsOf(withValues(valid, { host: '192.168.1.300' }))).toEqual(['host']);
  });

  it.each(['0', '65536', '22.5', 'abc', '', '-1'])('DEV-010: port %j is rejected when SSH is on', (port) => {
    const values = withValues(valid, { ssh: { enabled: true, port, username: 'root', auth: 'PASSWORD' } });
    expect(fieldsOf(values)).toEqual(['ssh.port']);
  });

  it.each(['1', '22', '65535'])('DEV-010: port %j is accepted', (port) => {
    const values = withValues(valid, { ssh: { enabled: true, port, username: 'root', auth: 'PASSWORD' } });
    expect(validateDevice(values)).toEqual([]);
  });

  it('DEV-010: an SSH user is required only when SSH is on', () => {
    expect(fieldsOf(withValues(valid, { ssh: { enabled: true, port: '22', username: ' ', auth: 'PASSWORD' } }))).toEqual(['ssh.username']);
    expect(validateDevice(withValues(valid, { ssh: { enabled: false, port: 'x', username: '', auth: 'PASSWORD' } }))).toEqual([]);
  });

  it('DEV-012: a password credential is optional', () => {
    const ssh = { enabled: true, port: '22', username: 'root', auth: 'PASSWORD' } as const;
    expect(validateDevice(withValues(valid, { ssh }))).toEqual([]);
  });

  it('DEV-012: a private key needs an armour line; an empty key keeps the stored one', () => {
    const ssh = { enabled: true, port: '22', username: 'root', auth: 'PRIVATE_KEY' } as const;
    const base = withValues(valid, { ssh });
    expect(validateDevice(base)).toEqual([]);
    expect(fieldsOf(withCredential(base, { privateKey: 'hunter2' }))).toEqual(['credential.privateKey.key']);
    expect(fieldsOf(withCredential(base, { privateKey: '-----BEGIN CERTIFICATE-----\nx' }))).toEqual(['credential.privateKey.key']);
    expect(validateDevice(withCredential(base, { privateKey: KEY }))).toEqual([]);
    expect(validateDevice(withCredential(base, { privateKey: '-----BEGIN RSA PRIVATE KEY-----\nx' }))).toEqual([]);
  });

  it('DEV-012: validation messages never repeat the secret', () => {
    const ssh = { enabled: true, port: '22', username: 'root', auth: 'PRIVATE_KEY' } as const;
    const errors = validateDevice(withCredential(withValues(valid, { ssh }), { privateKey: 'hunter2' }));
    expect(JSON.stringify(errors)).not.toContain('hunter2');
  });
});

describe('createInput', () => {
  it('DEV-010: sends trimmed values, clean tags and the default check', () => {
    const values = withValues(formValues(null), { name: ' nas ', host: ' [::1] ', tags: [' a ', 'a', '', 'b'] });
    expect(createInput(values, 'site-1')).toEqual({
      siteId: 'site-1',
      name: 'nas',
      host: '::1',
      kind: 'server',
      os: 'OTHER',
      tags: ['a', 'b'],
      addDefaultCheck: true,
    });
  });

  it('DEV-010: sends SSH settings only when SSH is on', () => {
    const values = withValues(formValues(null), {
      name: 'n',
      host: 'h',
      ssh: { enabled: true, port: '2200', username: ' root ', auth: 'PRIVATE_KEY' },
    });
    expect(createInput(values, 's').ssh).toEqual({ enabled: true, port: 2200, username: 'root', auth: 'PRIVATE_KEY' });
  });
});

describe('updateInput', () => {
  const initial = formValues(SAVED);

  it('DEV-010: an unchanged form gives an empty update', () => {
    expect(updateInput(initial, initial)).toEqual({});
  });

  it('DEV-010: whitespace and duplicate tags are not changes', () => {
    expect(updateInput(initial, withValues(initial, { name: ' nas ', tags: ['lab', ' storage ', 'lab'] }))).toEqual({});
  });

  it('DEV-010: only the changed fields are sent', () => {
    expect(updateInput(initial, withValues(initial, { host: '10.0.0.9' }))).toEqual({ host: '10.0.0.9' });
    expect(updateInput(initial, withValues(initial, { os: 'MACOS', tags: ['lab'] }))).toEqual({ os: 'MACOS', tags: ['lab'] });
  });

  it('DEV-010: a changed SSH field sends the whole SSH settings', () => {
    const changed = withValues(initial, { ssh: { ...initial.ssh, port: '22' } });
    expect(updateInput(initial, changed)).toEqual({
      ssh: { enabled: true, port: 22, username: 'root', auth: 'PASSWORD' },
    });
  });

  it('DEV-012: typing a credential alone does not change the device fields', () => {
    expect(updateInput(initial, withCredential(initial, { password: 'x' }))).toEqual({});
  });
});

describe('updateInput, one field at a time', () => {
  const initial = formValues(SAVED);
  it.each([
    ['name', { name: 'other' }, 'name'],
    ['kind', { kind: 'router' }, 'kind'],
    ['ssh.enabled', { ssh: { ...initial.ssh, enabled: false } }, 'ssh'],
    ['ssh.username', { ssh: { ...initial.ssh, username: 'admin' } }, 'ssh'],
    ['ssh.auth', { ssh: { ...initial.ssh, auth: 'PRIVATE_KEY' } }, 'ssh'],
  ] as const)('DEV-010: changing %s sends exactly one key', (_label, change, key) => {
    expect(Object.keys(updateInput(initial, withValues(initial, change)))).toEqual([key]);
  });

  it('DEV-010: with SSH off, a garbage port sends the last valid port, not null or 0', () => {
    const off = withValues(initial, { ssh: { ...initial.ssh, enabled: false, port: 'abc' } });
    expect(updateInput(initial, off)).toEqual({ ssh: { enabled: false, port: 2222, username: 'root', auth: 'PASSWORD' } });
  });

  it('DEV-010: with SSH off and no earlier port, a garbage port sends 22', () => {
    const off = withValues(formValues(null), { name: 'n', host: 'h', ssh: { enabled: false, port: '', username: '', auth: 'PASSWORD' } });
    expect(connectionTestInput(null, off, null).ssh?.port).toBe(22);
    expect(createInput(withValues(off, { ssh: { ...off.ssh, enabled: true, port: '2200', username: 'u' } }), 's').ssh?.port).toBe(2200);
  });
});

describe('private key armour', () => {
  const ssh = { enabled: true, port: '22', username: 'root', auth: 'PRIVATE_KEY' } as const;
  const base = withValues(withValues(formValues(null), { name: 'n', host: 'h' }), { ssh });

  it('DEV-012: key text under Password auth is not checked', () => {
    const password = withValues(base, { ssh: { ...ssh, auth: 'PASSWORD' } });
    expect(validateDevice(withCredential(password, { privateKey: 'stray text' }))).toEqual([]);
  });

  it.each(['x-----BEGIN PRIVATE KEY-----', '-----BEGIN PUBLIC KEY-----\nabc'])('DEV-012: %j is rejected', (text) => {
    expect(fieldsOf(withCredential(base, { privateKey: text }))).toEqual(['credential.privateKey.key']);
  });

  it('DEV-012: leading whitespace before the armour line is allowed', () => {
    expect(validateDevice(withCredential(base, { privateKey: `\n  ${KEY}` }))).toEqual([]);
  });
});


describe('credentialInput', () => {
  const password = withValues(formValues(null), { ssh: { enabled: true, port: '22', username: 'u', auth: 'PASSWORD' } });
  const privateKey = withValues(password, { ssh: { ...password.ssh, auth: 'PRIVATE_KEY' } });

  it('DEV-012: empty fields give null so the stored credential is kept', () => {
    expect(credentialInput(password)).toBeNull();
    expect(credentialInput(privateKey)).toBeNull();
    expect(credentialInput(withCredential(privateKey, { passphrase: 'only' }))).toBeNull();
  });

  it('DEV-012: a password is exactly one member', () => {
    expect(credentialInput(withCredential(password, { password: 's3cret' }))).toEqual({ password: 's3cret' });
  });

  it('DEV-012: a private key carries the key and an optional passphrase, and not the password', () => {
    const base = withCredential(privateKey, { privateKey: KEY, password: 'ignored' });
    expect(credentialInput(base)).toEqual({ privateKey: { key: KEY } });
    expect(credentialInput(withCredential(base, { passphrase: 'pp' }))).toEqual({ privateKey: { key: KEY, passphrase: 'pp' } });
  });

  it('DEV-012: nothing is sent while SSH is off', () => {
    expect(credentialInput(withCredential(formValues(null), { password: 'x' }))).toBeNull();
  });
});

describe('fieldErrorsFrom', () => {
  it('DEV-010: VALIDATION fields keep their camelCase paths', () => {
    const error: HubError = {
      code: 'VALIDATION',
      message: 'Input rejected.',
      fields: [
        { field: 'host', message: 'Not a host.' },
        { field: 'ssh.port', message: 'Out of range.' },
      ],
    };
    expect(fieldErrorsFrom(error)).toEqual(error.fields);
  });

  it.each([
    { code: 'CONFLICT', message: 'Name taken.' },
    { code: 'unreachable', message: 'The hub did not answer.' },
    { code: 'VALIDATION', message: 'Input rejected.', fields: [] },
  ] as const)('DEV-010: $code without fields is one form-level error with the hub message', (error) => {
    expect(fieldErrorsFrom(error)).toEqual([{ field: FORM_FIELD, message: error.message }]);
  });
});

describe('connectionTestInput', () => {
  const saved = formValues(SAVED);
  const ssh = saved.ssh;

  it('DEV-013: an unchanged saved device is tested by its ID alone', () => {
    expect(connectionTestInput(saved, saved, 'dev-1')).toEqual({ deviceId: 'dev-1' });
    expect(testsStoredCredential(saved, saved, 'dev-1')).toBe(true);
  });

  it('DEV-013: an unchanged saved device with a new credential sends the ID and the credential', () => {
    const typed = withCredential(saved, { password: 'new' });
    expect(connectionTestInput(saved, typed, 'dev-1')).toEqual({ deviceId: 'dev-1', credential: { password: 'new' } });
    expect(testsStoredCredential(saved, typed, 'dev-1')).toBe(false);
  });

  it.each([
    ['host', { host: '10.0.0.9' }],
    ['port', { ssh: { ...ssh, port: '22' } }],
    ['auth switch without a secret', { ssh: { ...ssh, auth: 'PRIVATE_KEY' } }],
  ] as const)('DEV-013: a changed %s sends the form values and no ID', (_label, change) => {
    const edited = withValues(saved, change);
    const input = connectionTestInput(saved, edited, 'dev-1');
    expect(input).not.toHaveProperty('deviceId');
    expect(input).toMatchObject({ host: edited.host, ssh: { port: Number(edited.ssh.port), auth: edited.ssh.auth } });
    expect(input).not.toHaveProperty('credential');
    expect(testsStoredCredential(saved, edited, 'dev-1')).toBe(false);
  });

  it('DEV-013: a changed host with a new credential sends both', () => {
    const edited = withCredential(withValues(saved, { host: '10.0.0.9' }), { password: 'new' });
    expect(connectionTestInput(saved, edited, 'dev-1')).toEqual({
      host: '10.0.0.9',
      ssh: { enabled: true, port: 2222, username: 'root', auth: 'PASSWORD' },
      credential: { password: 'new' },
    });
  });

  it('DEV-013: an unsaved device sends host and SSH, with no credential key when empty', () => {
    const values = withValues(formValues(null), { host: ' 10.0.0.5 ' });
    expect(connectionTestInput(null, values, null)).toEqual({
      host: '10.0.0.5',
      ssh: { enabled: false, port: 22, username: '', auth: 'PASSWORD' },
    });
    expect(testsStoredCredential(null, values, null)).toBe(false);
  });
});

describe('testOutcome', () => {
  const base = testResult({ reachable: true, authenticated: true, hostKeyChanged: false });

  it('DEV-013: reachable and authenticated is ok', () => {
    expect(testOutcome(base)).toEqual({ kind: 'ok' });
  });

  it('DEV-013: not reachable carries the hub message, or the default sentence', () => {
    expect(testOutcome({ ...base, reachable: false, authenticated: false, message: 'Timed out.' })).toEqual({ kind: 'unreachable', message: 'Timed out.' });
    expect(testOutcome({ ...base, reachable: false, authenticated: false })).toEqual({ kind: 'unreachable', message: 'The device did not answer.' });
  });

  it('DEV-013: reachable but refused is authFailed, with the hub message or the default sentence', () => {
    expect(testOutcome({ ...base, authenticated: false, message: 'Bad password.' })).toEqual({ kind: 'authFailed', message: 'Bad password.' });
    expect(testOutcome({ ...base, authenticated: false })).toEqual({ kind: 'authFailed', message: 'The device refused the credential.' });
  });

  it('SEC-006: a host key not yet accepted is hostKeyNew with the key', () => {
    expect(testOutcome({ ...base, authenticated: false, hostKey: HOST_KEY })).toEqual({ kind: 'hostKeyNew', hostKey: HOST_KEY });
  });

  it('SEC-007: a changed host key is hostKeyChanged with the key', () => {
    expect(testOutcome({ ...base, authenticated: false, hostKey: HOST_KEY, hostKeyChanged: true })).toEqual({ kind: 'hostKeyChanged', hostKey: HOST_KEY });
  });

  it('SEC-007: a changed host key without the key is never ok', () => {
    expect(testOutcome({ ...base, hostKeyChanged: true })).toEqual({ kind: 'hostKeyChangedUnknown' });
    expect(testOutcome({ ...base, hostKeyChanged: true, hostKey: null })).toEqual({ kind: 'hostKeyChangedUnknown' });
  });
});

describe('formValues', () => {
  it('D-4: every call gives a credential object of its own, so that editing one form cannot change another', () => {
    const first = formValues(null);
    const second = formValues(null);

    expect(first.credential).not.toBe(second.credential);
    expect(formValues(SAVED).credential).not.toBe(formValues(SAVED).credential);
  });
});

describe('testsSavedRecord', () => {
  it('SEC-007, D-5: holds for a saved device with its host and SSH settings as saved, and not once the host is edited or when unsaved', () => {
    const initial = formValues(SAVED);

    expect(testsSavedRecord(initial, initial, 'd1')).toBe(true);
    expect(testsSavedRecord(initial, withValues(initial, { host: 'other.home.arpa' }), 'd1')).toBe(false);
    expect(testsSavedRecord(initial, initial, null)).toBe(false);
    expect(testsSavedRecord(null, initial, null)).toBe(false);
  });
});
