import { describe, expect, it } from 'vitest';
import { fieldsOf, fresh } from './check.fixture';
import { isSecretHeader } from './model/config-input';
import { createCheckInput } from './model/input';
import type { CheckFormValues } from './model/values';

const repeat = (count: number, text = 'a') => text.repeat(count);

function http(change: (values: CheckFormValues['http']) => void): CheckFormValues {
  return fresh('HTTP', (form) => {
    form.http.url = 'https://example.com';
    change(form.http);
  });
}

describe('common settings follow the hub limits', () => {
  it.each([
    ['intervalSeconds', '9', ['intervalSeconds']],
    ['intervalSeconds', '10', []],
    ['intervalSeconds', '86400', []],
    ['intervalSeconds', '86401', ['intervalSeconds']],
    ['retries', '10', []],
    ['retries', '11', ['retries']],
    ['retryIntervalSeconds', '4', ['retryIntervalSeconds']],
    ['retryIntervalSeconds', '5', []],
    ['retryIntervalSeconds', '3600', []],
    ['retryIntervalSeconds', '3601', ['retryIntervalSeconds']],
    ['degradedThresholdMs', '0', ['degradedThresholdMs']],
    ['degradedThresholdMs', '1', []],
    ['degradedThresholdMs', '600000', []],
    ['degradedThresholdMs', '600001', ['degradedThresholdMs']],
  ] as const)('CHK-009: %s %s gives %j', (field, text, expected) => {
    expect(fieldsOf(fresh('ICMP', (form) => (form[field] = text)))).toEqual(expected);
  });

  it('CHK-009: a name has at most 200 characters', () => {
    expect(fieldsOf(fresh('ICMP', (form) => (form.name = repeat(200))))).toEqual([]);
    expect(fieldsOf(fresh('ICMP', (form) => (form.name = repeat(201))))).toEqual(['name']);
  });
});

describe('HTTP text limits', () => {
  it('CHK-009: a URL has at most 2048 bytes', () => {
    expect(fieldsOf(http((h) => (h.url = `https://example.com/${repeat(2048 - 20)}`)))).toEqual([]);
    expect(fieldsOf(http((h) => (h.url = `https://example.com/${repeat(2048 - 19)}`)))).toEqual(['url']);
  });

  it('CHK-009: a body has at most 64 KiB, a keyword and an expected value 1024 bytes', () => {
    expect(fieldsOf(http((h) => (h.body = repeat(65_536))))).toEqual([]);
    expect(fieldsOf(http((h) => (h.body = repeat(65_537))))).toEqual(['body']);
    expect(fieldsOf(http((h) => (h.keyword = repeat(1025))))).toEqual(['keyword']);
    expect(fieldsOf(http((h) => (h.keyword = repeat(1024))))).toEqual([]);
    const long = (h: CheckFormValues['http']) => {
      h.jsonQuery = '/a';
      h.jsonExpected = repeat(1025);
    };
    expect(fieldsOf(http(long))).toEqual(['jsonExpected']);
  });

  it('CHK-009: an expected JSON value needs the pointer to compare, and a pointer starts with a slash', () => {
    expect(fieldsOf(http((h) => (h.jsonExpected = 'ok')))).toEqual(['jsonExpected']);
    expect(fieldsOf(http((h) => ((h.jsonQuery = '/status'), (h.jsonExpected = 'ok'))))).toEqual([]);
    expect(fieldsOf(http((h) => (h.jsonQuery = 'status')))).toEqual(['jsonQuery']);
    expect(fieldsOf(http((h) => (h.jsonQuery = `/${repeat(512)}`)))).toEqual(['jsonQuery']);
  });

  it('CHK-009: a basic user name has no colon and at most 256 bytes', () => {
    const basic = (username: string) => http((h) => ((h.authKind = 'BASIC'), (h.username = username), (h.password = 'pw')));
    expect(fieldsOf(basic('probe'))).toEqual([]);
    expect(fieldsOf(basic('a:b'))).toEqual(['username']);
    expect(fieldsOf(basic(repeat(256)))).toEqual([]);
    expect(fieldsOf(basic(repeat(257)))).toEqual(['username']);
  });

  it('CHK-009: a header name has at most 128 bytes and is not a destination header', () => {
    const row = (name: string) => http((h) => (h.headers = [{ name, value: 'v', stored: false }]));
    expect(fieldsOf(row(repeat(128)))).toEqual([]);
    expect(fieldsOf(row(repeat(129)))).toEqual(['headers.0.name']);
    expect(fieldsOf(row('Host'))).toEqual(['headers.0.name']);
  });

  it('CHK-009: a header value has no line break', () => {
    const row = (value: string) => http((h) => (h.headers = [{ name: 'X-A', value, stored: false }]));
    expect(fieldsOf(row('a\nb'))).toEqual(['headers.0.value']);
    expect(fieldsOf(row('a\rb'))).toEqual(['headers.0.value']);
    expect(fieldsOf(row('a b'))).toEqual([]);
  });
});

describe('counts', () => {
  const headers = (count: number) => Array.from({ length: count }, (_, index) => ({ name: `X-${String(index)}`, value: 'v', stored: false }));
  const ranges = (count: number) => Array.from({ length: count }, () => ({ from: '200', to: '299' }));

  it('CHK-009: at most 20 headers', () => {
    expect(fieldsOf(http((h) => (h.headers = headers(20))))).toEqual([]);
    expect(fieldsOf(http((h) => (h.headers = headers(21))))).toEqual(['headers']);
  });

  it('CHK-009: blank header rows are not counted and not sent', () => {
    const rows = [...headers(20), { name: '', value: '', stored: false }, { name: ' ', value: '', stored: false }];
    const values = http((h) => (h.headers = rows));
    expect(fieldsOf(values)).toEqual([]);
    expect(createCheckInput(values, 'd').config).toMatchObject({ http: { headers: headers(20).map(({ name, value }) => ({ name, value })) } });
  });

  it('CHK-009: at most 20 accepted status ranges', () => {
    expect(fieldsOf(http((h) => (h.acceptedStatus = ranges(20))))).toEqual([]);
    expect(fieldsOf(http((h) => (h.acceptedStatus = ranges(21))))).toEqual(['acceptedStatus']);
  });

  it('CHK-009: a group takes at most 100 checks', () => {
    const ids = (count: number) => Array.from({ length: count }, (_, index) => `c${String(index)}`);
    expect(fieldsOf(fresh('GROUP', (form) => (form.group.childIds = ids(100))))).toEqual([]);
    expect(fieldsOf(fresh('GROUP', (form) => (form.group.childIds = ids(101))))).toEqual(['childIds']);
  });

  it('CHK-009: an expected DNS answer has at most 1024 bytes', () => {
    const dns = (expected: string) =>
      fresh('DNS', (form) => {
        form.dns.name = 'example.com';
        form.dns.expected = expected;
      });
    expect(fieldsOf(dns(repeat(1024)))).toEqual([]);
    expect(fieldsOf(dns(repeat(1025)))).toEqual(['expected']);
  });
});

describe('secret header names', () => {
  it.each(['Authorization', 'authorization', 'Proxy-Authorization', 'COOKIE', ' Cookie '])('CHK-009: %j is secret', (name) => {
    expect(isSecretHeader(name)).toBe(true);
  });

  it.each(['X-Api-Key', 'Set-Cookie', 'Authorization2'])('CHK-009: %j is not secret', (name) => {
    expect(isSecretHeader(name)).toBe(false);
  });

  it('CHK-009: a new Proxy-Authorization header needs a value', () => {
    const values = http((h) => (h.headers = [{ name: 'Proxy-Authorization', value: '', stored: false }]));
    expect(fieldsOf(values)).toEqual(['headers.0.value']);
  });
});

describe('DNS resolver', () => {
  it('CHK-009: a bracketed IPv6 resolver is sent bare and a blank one is left out', () => {
    const dns = (resolver: string) =>
      createCheckInput(
        fresh('DNS', (form) => {
          form.dns.name = ' example.com ';
          form.dns.resolver = resolver;
        }),
        'd',
      ).config;
    expect(dns(' [2606:4700::1111] ')).toEqual({ dns: { name: 'example.com', recordType: 'A', resolver: '2606:4700::1111' } });
    expect(dns('  ')).toEqual({ dns: { name: 'example.com', recordType: 'A' } });
  });

  it('CHK-009: a bracketed IPv6 resolver is valid', () => {
    expect(fieldsOf(fresh('DNS', (form) => ((form.dns.name = 'example.com'), (form.dns.resolver = '[::1]'))))).toEqual([]);
  });
});
