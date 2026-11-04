import { describe, expect, it } from 'vitest';
import { BEARER_SAVED, fieldsOf, fresh } from './check.fixture';
import { createCheckInput, updateCheckInput } from './model/input';
import { checkValues } from './model/values';

describe('bearer token', () => {
  it('CHK-009: a new check sends the typed token', () => {
    const values = fresh('HTTP', (form) => {
      form.http.url = 'https://example.com';
      form.http.authKind = 'BEARER';
      form.http.bearerToken = 'tok-1';
    });
    expect(createCheckInput(values, 'dev-1').config).toMatchObject({ http: { auth: { bearerToken: 'tok-1' } } });
    expect(fieldsOf(values)).toEqual([]);
  });

  it('CHK-009: a new check with no token is refused', () => {
    const values = fresh('HTTP', (form) => {
      form.http.url = 'https://example.com';
      form.http.authKind = 'BEARER';
    });
    expect(fieldsOf(values)).toEqual(['bearerToken']);
  });

  it('CHK-009: an edit sends a newly typed token', () => {
    const initial = checkValues(BEARER_SAVED, 'ICMP');
    const edited = checkValues(BEARER_SAVED, 'ICMP');
    edited.http.bearerToken = 'tok-2';
    expect(updateCheckInput(initial, edited).config).toMatchObject({ http: { auth: { bearerToken: 'tok-2' } } });
  });

  it('CHK-009: an empty token on an unchanged kind is sent empty, which keeps the stored one', () => {
    const initial = checkValues(BEARER_SAVED, 'ICMP');
    const edited = checkValues(BEARER_SAVED, 'ICMP');
    edited.http.timeoutMs = '5000';
    const update = updateCheckInput(initial, edited);
    expect(update.config).toMatchObject({ http: { timeoutMs: 5000, auth: { bearerToken: '' } } });
    expect(fieldsOf(edited, initial)).toEqual([]);
  });

  it('CHK-009: an edit that does not touch the config sends no token at all', () => {
    const initial = checkValues(BEARER_SAVED, 'ICMP');
    const edited = checkValues(BEARER_SAVED, 'ICMP');
    edited.name = 'renamed';
    expect(updateCheckInput(initial, edited)).toEqual({ name: 'renamed' });
  });

  it('CHK-009: switching to NONE sends no authentication', () => {
    const initial = checkValues(BEARER_SAVED, 'ICMP');
    const edited = checkValues(BEARER_SAVED, 'ICMP');
    edited.http.authKind = 'NONE';
    const config = updateCheckInput(initial, edited).config;
    expect(config).toHaveProperty('http.url');
    expect(config).not.toHaveProperty('http.auth');
  });
});
