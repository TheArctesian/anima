import { expect, it } from 'vitest';
import { checkConnect, restorePem } from './connect';

const FORM = { label: 'Personal', appId: '12345', installationId: '678', privateKey: '-----BEGIN RSA PRIVATE KEY-----\nAAAA\n-----END RSA PRIVATE KEY-----' };

it('PRJ-018: a valid form becomes the hub input with the key as it was pasted', () => {
  expect(checkConnect(FORM)).toEqual({ ok: true, input: { label: 'Personal', appId: '12345', installationId: '678', privateKey: FORM.privateKey } });
});

it('PRJ-018: each wrong field is named, and a problem message never holds the key', () => {
  const checked = checkConnect({ label: '', appId: 'abc', installationId: '', privateKey: 'SECRET-TEXT' });
  expect(checked.ok).toBe(false);
  if (checked.ok) return;
  expect(checked.problems.map((each) => each.field)).toEqual(['label', 'appId', 'installationId']);
  expect(JSON.stringify(checked.problems)).not.toContain('SECRET-TEXT');
});

it('PRJ-018: a key whose line breaks a password field removed is put back in lines of 64', () => {
  const flat = `-----BEGIN RSA PRIVATE KEY----- ${'A'.repeat(130)} -----END RSA PRIVATE KEY-----`;
  expect(restorePem(flat).split('\n')).toEqual(['-----BEGIN RSA PRIVATE KEY-----', 'A'.repeat(64), 'A'.repeat(64), 'AA', '-----END RSA PRIVATE KEY-----']);
});
