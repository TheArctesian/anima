import { describe, expect, it } from 'vitest';
import { fieldsOf, fresh } from './check.fixture';

describe('validation', () => {
  it('CHK-009: a name, an ICMP timeout and the defaults make a valid form', () => {
    expect(fieldsOf(fresh('ICMP'))).toEqual([]);
  });

  it.each([
    ['intervalSeconds', '0'],
    ['intervalSeconds', 'abc'],
    ['retries', '-1'],
    ['retryIntervalSeconds', '0'],
    ['degradedThresholdMs', ''],
  ] as const)('CHK-009: %s %j is refused', (field, text) => {
    const values = fresh('ICMP', (form) => {
      form[field] = text;
    });
    expect(fieldsOf(values)).toEqual([field]);
  });

  it('CHK-009: zero retries are allowed', () => {
    expect(fieldsOf(fresh('ICMP', (form) => (form.retries = '0')))).toEqual([]);
  });

  it('CHK-009: a name is required', () => {
    expect(fieldsOf(fresh('ICMP', (form) => (form.name = '  ')))).toEqual(['name']);
  });

  it.each([
    ['', ['port']],
    ['0', ['port']],
    ['65536', ['port']],
    ['65535', []],
    ['1', []],
    ['22x', ['port']],
  ])('CHK-009: TCP port %j gives %j', (port, expected) => {
    expect(fieldsOf(fresh('TCP', (form) => (form.tcp.port = port)))).toEqual(expected);
  });

  it('CHK-009: a timeout below 100 ms or above 60 s is refused', () => {
    expect(fieldsOf(fresh('ICMP', (form) => (form.icmp.timeoutMs = '99')))).toEqual(['timeoutMs']);
    expect(fieldsOf(fresh('ICMP', (form) => (form.icmp.timeoutMs = '60001')))).toEqual(['timeoutMs']);
  });

  it('CHK-009: a TLS server name must be a host name', () => {
    expect(fieldsOf(fresh('TLS', (form) => (form.tls.serverName = 'not a host')))).toEqual(['serverName']);
    expect(fieldsOf(fresh('TLS', (form) => (form.tls.serverName = 'a.example')))).toEqual([]);
  });

});

describe('validation of HTTP settings', () => {
  it.each([
    ['https://example.com', []],
    ['http://10.0.0.1:8080/x', []],
    ['', ['url']],
    ['ftp://example.com', ['url']],
    ['example.com', ['url']],
    ['https://user:pass@example.com', ['url']],
  ])('CHK-009: HTTP url %j gives %j', (url, expected) => {
    expect(fieldsOf(fresh('HTTP', (form) => (form.http.url = url)))).toEqual(expected);
  });

  it.each([
    [[{ from: '200', to: '299' }], []],
    [[{ from: '404', to: '404' }], []],
    [[{ from: '300', to: '200' }], ['acceptedStatus.0']],
    [[{ from: '99', to: '200' }], ['acceptedStatus.0']],
    [[{ from: '200', to: '600' }], ['acceptedStatus.0']],
    [[{ from: 'a', to: '200' }], ['acceptedStatus.0']],
    [[], ['acceptedStatus']],
  ])('CHK-009: HTTP status ranges %j give %j', (ranges, expected) => {
    const values = fresh('HTTP', (form) => {
      form.http.url = 'https://example.com';
      form.http.acceptedStatus = ranges;
    });
    expect(fieldsOf(values)).toEqual(expected);
  });

  it.each([
    ['X-Env', 'prod', []],
    ['', 'prod', ['headers.0.name']],
    ['Bad Name', 'prod', ['headers.0.name']],
    ['X:Y', 'prod', ['headers.0.name']],
  ])('CHK-009: header %j: %j gives %j', (name, value, expected) => {
    const values = fresh('HTTP', (form) => {
      form.http.url = 'https://example.com';
      form.http.headers = [{ name, value, stored: false }];
    });
    expect(fieldsOf(values)).toEqual(expected);
  });

  it('CHK-009: a redirect count above ten is refused', () => {
    const values = fresh('HTTP', (form) => {
      form.http.url = 'https://example.com';
      form.http.maxRedirects = '11';
    });
    expect(fieldsOf(values)).toEqual(['maxRedirects']);
  });

});

describe('validation of DNS settings', () => {
  it.each([
    ['example.com', '', []],
    ['', '', ['dnsName']],
    ['bad name', '', ['dnsName']],
    ['example.com', '1.1.1.1', []],
    ['example.com', '2606:4700::1111', []],
    ['example.com', 'dns.google', ['resolver']],
    ['example.com', '999.1.1.1', ['resolver']],
  ])('CHK-009: DNS name %j with resolver %j gives %j', (name, resolver, expected) => {
    const values = fresh('DNS', (form) => {
      form.dns.name = name;
      form.dns.resolver = resolver;
    });
    expect(fieldsOf(values)).toEqual(expected);
  });
});
