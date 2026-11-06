import { describe, expect, it } from 'vitest';
import { fieldsOf, fresh, HTTP_SAVED } from './check.fixture';
import { updateCheckInput } from './model/input';
import { validateCheck } from './model/validate';
import { checkValues } from './model/values';
import { secretWarnings } from './model/warnings';

describe('HTTP secrets', () => {
  it('CHK-009: no secret is ever filled from a saved check', () => {
    const { http } = checkValues(HTTP_SAVED, 'ICMP');
    expect(http).toMatchObject({ authKind: 'BASIC', storedAuth: 'BASIC', username: '', password: '', bearerToken: '' });
    expect(http.headers).toEqual([
      { name: 'X-Env', value: 'prod', stored: false },
      { name: 'Authorization', value: '', stored: true },
    ]);
  });

  it('CHK-009: an edit of the name leaves the stored secrets alone: no config is sent', () => {
    const initial = checkValues(HTTP_SAVED, 'ICMP');
    const edited = checkValues(HTTP_SAVED, 'ICMP');
    edited.name = 'renamed';
    expect(updateCheckInput(initial, edited)).toEqual({ name: 'renamed' });
    expect(validateCheck(edited, initial)).toEqual([]);
  });

  it('CHK-009: an empty password is sent as empty, which the hub reads as "keep the stored one"', () => {
    const initial = checkValues(HTTP_SAVED, 'ICMP');
    const edited = checkValues(HTTP_SAVED, 'ICMP');
    edited.http.username = 'probe';
    edited.http.timeoutMs = '5000';
    const update = updateCheckInput(initial, edited);
    expect(update.config).toMatchObject({
      http: {
        timeoutMs: 5000,
        auth: { basic: { username: 'probe', password: '' } },
        headers: [{ name: 'X-Env', value: 'prod' }, { name: 'Authorization', value: '' }],
      },
    });
    expect(validateCheck(edited, initial)).toEqual([]);
  });

  it('CHK-009: a changed config needs the stored user name again, since the hub never returns it', () => {
    const initial = checkValues(HTTP_SAVED, 'ICMP');
    const edited = checkValues(HTTP_SAVED, 'ICMP');
    edited.http.timeoutMs = '5000';
    expect(fieldsOf(edited, initial)).toEqual(['username']);
  });

  it('CHK-009: a new secret must be entered in create mode and when the kind changes', () => {
    const create = fresh('HTTP', (form) => {
      form.http.url = 'https://example.com';
      form.http.authKind = 'BEARER';
    });
    expect(fieldsOf(create)).toEqual(['bearerToken']);
    const initial = checkValues(HTTP_SAVED, 'ICMP');
    const edited = checkValues(HTTP_SAVED, 'ICMP');
    edited.http.authKind = 'BEARER';
    expect(fieldsOf(edited, initial)).toEqual(['bearerToken']);
  });

  it('CHK-009: a new Authorization header needs a value; a stored one does not', () => {
    const create = fresh('HTTP', (form) => {
      form.http.url = 'https://example.com';
      form.http.headers = [{ name: 'Cookie', value: '', stored: false }];
    });
    expect(fieldsOf(create)).toEqual(['headers.0.value']);
  });

});

describe('HTTP secret warnings', () => {
  it('CHK-009: warns when the address changes while a stored secret is kept', () => {
    const initial = checkValues(HTTP_SAVED, 'ICMP');
    const edited = checkValues(HTTP_SAVED, 'ICMP');
    edited.http.url = 'https://other.example/health';
    edited.http.username = 'probe';
    expect(secretWarnings(initial, edited)).toEqual([expect.stringContaining('will not reuse the stored secrets')]);
  });

  it('CHK-009: warns when the authentication or a secret header is removed', () => {
    const initial = checkValues(HTTP_SAVED, 'ICMP');
    const edited = checkValues(HTTP_SAVED, 'ICMP');
    edited.http.authKind = 'NONE';
    edited.http.headers = edited.http.headers.slice(0, 1);
    expect(secretWarnings(initial, edited)).toEqual([
      expect.stringContaining('removes the stored authentication'),
      expect.stringContaining('authorization header'),
    ]);
  });

  it('CHK-009: no warning when the config is not sent', () => {
    const initial = checkValues(HTTP_SAVED, 'ICMP');
    expect(secretWarnings(initial, checkValues(HTTP_SAVED, 'ICMP'))).toEqual([]);
  });
});
