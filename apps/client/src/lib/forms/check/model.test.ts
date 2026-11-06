import { describe, expect, it } from 'vitest';
import { fresh, saved } from './check.fixture';
import { configInput } from './model/config-input';
import { groupCandidates, siblingOf, type SiblingCheck } from './model/group';
import { createCheckInput, updateCheckInput } from './model/input';
import { checkValues } from './model/values';

const DEVICE = 'dev-1';

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
    [saved('GROUP', { __typename: 'GroupConfig', children: [{ id: 'b', name: 'b' }, { id: 'a', name: 'a' }] }), { group: { childIds: ['a', 'b'] } }],
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
    const check = saved('GROUP', { __typename: 'GroupConfig', children: [{ id: 'z', name: 'z' }, { id: 'a', name: 'a' }] });
    expect(configInput(checkValues(check, 'ICMP'))).toEqual({ group: { childIds: ['a', 'z'] } });
    expect(siblingOf(check)).toEqual({ id: 'chk-1', name: 'check', type: 'GROUP', childIds: ['z', 'a'] });
  });
});
