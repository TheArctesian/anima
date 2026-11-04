import { describe, expect, it } from 'vitest';
import { configInput } from './model/config-input';
import { groupCandidates, siblingOf, type SiblingCheck } from './model/group';
import type { CheckConfigFields, CheckFields, CheckType } from './model/hub-types';
import { createCheckInput, updateCheckInput } from './model/input';
import { validateCheck } from './model/validate';
import { secretWarnings } from './model/warnings';
import { checkValues, type CheckFormValues } from './model/values';

const DEVICE = 'dev-1';

function saved(type: CheckType, config: CheckConfigFields, change: Partial<CheckFields> = {}): CheckFields {
  return {
    id: 'chk-1',
    type,
    name: 'check',
    intervalSeconds: 60,
    retries: 3,
    retryIntervalSeconds: 20,
    degradedThresholdMs: 1000,
    invert: false,
    config,
    ...change,
  };
}

const HTTP_SAVED = saved('HTTP', {
  __typename: 'HttpConfig',
  url: 'https://example.com/health',
  method: 'GET',
  headers: [
    { name: 'X-Env', value: 'prod' },
    { name: 'Authorization', value: null },
  ],
  body: null,
  maxRedirects: 10,
  acceptedStatus: [{ from: 200, to: 299 }],
  keyword: null,
  jsonQuery: null,
  jsonExpected: null,
  authKind: 'BASIC',
  verifyTls: true,
  timeoutMs: 10000,
});

function fresh(type: CheckType, change: (values: CheckFormValues) => void = () => undefined): CheckFormValues {
  const values = checkValues(null, type);
  values.name = 'a check';
  change(values);
  return values;
}

function fieldsOf(values: CheckFormValues, initial: CheckFormValues | null = null): string[] {
  return validateCheck(values, initial).map((error) => error.field);
}

describe('create input with K1 defaults', () => {
  const common = { deviceId: DEVICE, name: 'a check', intervalSeconds: 60, retries: 3, retryIntervalSeconds: 20, degradedThresholdMs: 1000, invert: false };

  it.each([
    ['ICMP', {}, { icmp: { timeoutMs: 2000, ipVersion: 'V4' } }],
    ['TCP', { tcp: 22 }, { tcp: { port: 22, timeoutMs: 5000 } }],
    ['TLS', {}, { tls: { port: 443, timeoutMs: 5000 } }],
    ['PUSH', {}, { push: true }],
    ['GROUP', {}, { group: { childIds: [] } }],
  ] as const)('CHK-009: a new %s check carries the K1 defaults', (type, options, config) => {
    const values = fresh(type, (form) => {
      if ('tcp' in options) form.tcp.port = String(options.tcp);
    });
    expect(createCheckInput(values, DEVICE)).toEqual({ ...common, config });
  });

  it('CHK-009: a new HTTP check defaults to GET, ten redirects, 200-299, verified TLS and 10 s', () => {
    const values = fresh('HTTP', (form) => {
      form.http.url = ' https://example.com ';
    });
    expect(createCheckInput(values, DEVICE).config).toEqual({
      http: {
        url: 'https://example.com',
        method: 'GET',
        headers: [],
        maxRedirects: 10,
        acceptedStatus: [{ from: 200, to: 299 }],
        verifyTls: true,
        timeoutMs: 10000,
      },
    });
  });

  it('CHK-009: a new DNS check defaults to record type A and drops an empty resolver', () => {
    const values = fresh('DNS', (form) => {
      form.dns.name = 'example.com';
    });
    expect(createCheckInput(values, DEVICE).config).toEqual({ dns: { name: 'example.com', recordType: 'A' } });
  });

  it('CHK-008: invert is sent as chosen', () => {
    const values = fresh('TCP', (form) => {
      form.tcp.port = '25';
      form.invert = true;
    });
    expect(createCheckInput(values, DEVICE).invert).toBe(true);
  });
});

describe('edit round trip', () => {
  it.each([
    [saved('ICMP', { __typename: 'IcmpConfig', timeoutMs: 1500, ipVersion: 'V6' }), { icmp: { timeoutMs: 1500, ipVersion: 'V6' } }],
    [saved('TCP', { __typename: 'TcpConfig', port: 22, timeoutMs: 3000 }), { tcp: { port: 22, timeoutMs: 3000 } }],
    [saved('TLS', { __typename: 'TlsConfig', port: 8443, serverName: 'a.example', timeoutMs: 5000 }), { tls: { port: 8443, serverName: 'a.example', timeoutMs: 5000 } }],
    [saved('DNS', { __typename: 'DnsConfig', name: 'a.example', recordType: 'MX', resolver: '1.1.1.1', expected: null }), { dns: { name: 'a.example', recordType: 'MX', resolver: '1.1.1.1' } }],
    [saved('PUSH', { __typename: 'PushConfig', pushPath: '/push/abc' }), { push: true }],
    [saved('GROUP', { __typename: 'GroupConfig', children: [{ id: 'b' }, { id: 'a' }] }), { group: { childIds: ['a', 'b'] } }],
  ] as const)('CHK-009: a saved %s check gives back its own config and no changes', (check, config) => {
    const initial = checkValues(check, 'ICMP');
    expect(configInput(initial)).toEqual(config);
    expect(updateCheckInput(initial, checkValues(check, 'ICMP'))).toEqual({});
  });

  it('CHK-009: the push address is shown from the saved check', () => {
    expect(checkValues(saved('PUSH', { __typename: 'PushConfig', pushPath: '/push/abc' }), 'ICMP').push.path).toBe('/push/abc');
  });
});

describe('update input', () => {
  const check = saved('TCP', { __typename: 'TcpConfig', port: 22, timeoutMs: 3000 });

  it('CHK-009: only changed fields are sent', () => {
    const initial = checkValues(check, 'ICMP');
    const edited = checkValues(check, 'ICMP');
    edited.name = 'ssh';
    edited.retries = '5';
    expect(updateCheckInput(initial, edited)).toEqual({ name: 'ssh', retries: 5 });
  });

  it('CHK-009: a changed config is sent whole', () => {
    const initial = checkValues(check, 'ICMP');
    const edited = checkValues(check, 'ICMP');
    edited.tcp.port = '2222';
    expect(updateCheckInput(initial, edited)).toEqual({ config: { tcp: { port: 2222, timeoutMs: 3000 } } });
  });

  it('CHK-008: invert on is sent as a change', () => {
    const initial = checkValues(check, 'ICMP');
    const edited = checkValues(check, 'ICMP');
    edited.invert = true;
    expect(updateCheckInput(initial, edited)).toEqual({ invert: true });
  });

  it('CHK-009: type fixed after create; a changed type in the values is ignored', () => {
    const initial = checkValues(check, 'ICMP');
    const edited = checkValues(check, 'ICMP');
    edited.type = 'ICMP';
    edited.name = 'renamed';
    expect(updateCheckInput(initial, edited)).toEqual({ name: 'renamed' });
    expect(checkValues(check, 'HTTP').type).toBe('TCP');
  });
});

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

describe('group children', () => {
  const sibling = (id: string, childIds: string[] = []): SiblingCheck => ({ id, name: id, type: childIds.length > 0 ? 'GROUP' : 'TCP', childIds });

  it('CHK-009: a group never offers itself', () => {
    expect(groupCandidates('a', [sibling('a'), sibling('b')]).map((entry) => entry.id)).toEqual(['b']);
  });

  it('CHK-009: a group never offers a group that holds it, directly or through another group', () => {
    const siblings = [sibling('self'), sibling('direct', ['self']), sibling('outer', ['direct']), sibling('other', ['x']), sibling('leaf')];
    expect(groupCandidates('self', siblings).map((entry) => entry.id)).toEqual(['other', 'leaf']);
  });

  it('CHK-009: a group being created may hold every other check', () => {
    expect(groupCandidates(null, [sibling('a', ['b']), sibling('b')])).toHaveLength(2);
  });

  it('CHK-009: a saved group lists its children, sorted, in the input', () => {
    const check = saved('GROUP', { __typename: 'GroupConfig', children: [{ id: 'z' }, { id: 'a' }] });
    expect(configInput(checkValues(check, 'ICMP'))).toEqual({ group: { childIds: ['a', 'z'] } });
    expect(siblingOf(check)).toEqual({ id: 'chk-1', name: 'check', type: 'GROUP', childIds: ['z', 'a'] });
  });
});
