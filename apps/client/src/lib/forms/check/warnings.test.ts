import { describe, expect, it } from 'vitest';
import { BEARER_SAVED, HTTP_SAVED, saved } from './check.fixture';
import { secretWarnings } from './model/warnings';
import { checkValues, type CheckFormValues } from './model/values';

const REUSE = 'will not reuse the stored secrets';

/** HTTP_SAVED holds basic auth and a stored Authorization header; the user name is typed again. */
function edit(change: (values: CheckFormValues) => void): string[] {
  const initial = checkValues(HTTP_SAVED, 'ICMP');
  const values = checkValues(HTTP_SAVED, 'ICMP');
  values.http.username = 'probe';
  change(values);
  return secretWarnings(initial, values);
}

describe('secret warnings', () => {
  it('CHK-009: an edit that sends no config warns of nothing', () => {
    const initial = checkValues(HTTP_SAVED, 'ICMP');
    expect(secretWarnings(initial, checkValues(HTTP_SAVED, 'ICMP'))).toEqual([]);
  });

  it('CHK-009: a check that is not HTTP warns of nothing', () => {
    const tcp = saved('TCP', { __typename: 'TcpConfig', port: 22, timeoutMs: 3000 });
    const initial = checkValues(tcp, 'ICMP');
    const values = checkValues(tcp, 'ICMP');
    values.tcp.port = '23';
    expect(secretWarnings(initial, values)).toEqual([]);
  });

  it('CHK-009: a change that keeps the destination warns of nothing', () => {
    expect(edit((values) => (values.http.timeoutMs = '5000'))).toEqual([]);
  });

  it('CHK-009: removing the authentication warns', () => {
    const warnings = edit((values) => (values.http.authKind = 'NONE'));
    expect(warnings).toContainEqual(expect.stringContaining('removes the stored authentication'));
  });

  it('CHK-009: removing a stored secret header warns and names it', () => {
    const warnings = edit((values) => (values.http.headers = values.http.headers.filter((row) => !row.stored)));
    expect(warnings).toContainEqual(expect.stringContaining('authorization header'));
  });

});

describe('secret warnings when the destination changes', () => {
  it('CHK-009: a new address warns while a secret is kept', () => {
    expect(edit((values) => (values.http.url = 'https://other.example/'))).toEqual([expect.stringContaining(REUSE)]);
  });

  it('CHK-009: the same origin with another path warns of nothing', () => {
    expect(edit((values) => (values.http.url = 'https://example.com/other'))).toEqual([]);
  });

  it('CHK-009: turning TLS verification off warns while a secret is kept', () => {
    expect(edit((values) => (values.http.verifyTls = false))).toEqual([expect.stringContaining(REUSE)]);
  });

  it('CHK-009: turning TLS verification on warns of nothing', () => {
    const initial = checkValues(HTTP_SAVED, 'ICMP');
    initial.http.verifyTls = false;
    const values = checkValues(HTTP_SAVED, 'ICMP');
    values.http.username = 'probe';
    expect(secretWarnings(initial, values)).toEqual([]);
  });

  it.each(['Cookie', 'Proxy-Authorization', 'authorization'])(
    'CHK-009: a new %s header warns while a stored secret is kept',
    (name) => {
      const warnings = edit((values) => values.http.headers.push({ name, value: 'x', stored: false }));
      expect(warnings).toEqual([expect.stringContaining(REUSE)]);
    },
  );

  it('CHK-009: a new header that is not secret warns of nothing', () => {
    expect(edit((values) => values.http.headers.push({ name: 'X-Trace', value: 'x', stored: false }))).toEqual([]);
  });

  it('CHK-009: a new secret header with no stored secret kept warns of nothing', () => {
    const initial = checkValues(BEARER_SAVED, 'ICMP');
    initial.http.headers = [];
    initial.http.authKind = 'NONE';
    initial.http.storedAuth = 'NONE';
    const values = structuredClone(initial);
    values.http.headers = [{ name: 'Cookie', value: 'x', stored: false }];
    values.http.url = 'https://other.example';
    expect(secretWarnings(initial, values)).toEqual([]);
  });

  it('CHK-009: a bearer token kept with a new address warns', () => {
    const initial = checkValues(BEARER_SAVED, 'ICMP');
    const values = checkValues(BEARER_SAVED, 'ICMP');
    values.http.url = 'https://other.example';
    expect(secretWarnings(initial, values)).toEqual([expect.stringContaining(REUSE)]);
  });

  it('CHK-009: every secret typed again with a new address warns of nothing', () => {
    const warnings = edit((values) => {
      values.http.url = 'https://other.example/';
      values.http.password = 'pw';
      values.http.headers = values.http.headers.map((row) => (row.stored ? { ...row, value: 'secret' } : row));
    });
    expect(warnings).toEqual([]);
  });
});
