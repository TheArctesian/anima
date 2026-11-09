import { describe, expect, it } from 'vitest';
import { saved } from './check.fixture';
import { createCheckInput, updateCheckInput } from './model/input';
import { validateCheck } from './model/validate';
import { checkValues } from './model/values';

describe('update keeps the type of the saved check', () => {
  const tcp = saved('TCP', { __typename: 'TcpConfig', port: 22, timeoutMs: 3000 });

  it('CHK-009: type fixed after create: a config change is built for the saved type even if the values say another', () => {
    const initial = checkValues(tcp, 'ICMP');
    const edited = checkValues(tcp, 'ICMP');
    edited.tcp.port = '2222';
    edited.type = 'HTTP';

    expect(updateCheckInput(initial, edited)).toEqual({ config: { tcp: { port: 2222, timeoutMs: 3000 } } });
  });

  it('CHK-009: type fixed after create: a changed type alone changes nothing', () => {
    const initial = checkValues(tcp, 'ICMP');
    const edited = checkValues(tcp, 'ICMP');
    edited.type = 'DNS';
    edited.dns.name = 'example.com';

    expect(updateCheckInput(initial, edited)).toEqual({});
  });
});

describe('CHK-009, D-29: update of a DOMAIN_EXPIRY check', () => {
  const loaded = saved('DOMAIN_EXPIRY', { __typename: 'DomainExpiryConfig', domain: 'example.net', warnDays: 30, timeoutMs: 10000 }, { intervalSeconds: 43_200 });

  it('an unchanged config sends no config', () => {
    const edited = { ...checkValues(loaded, 'ICMP'), name: 'renamed' };
    expect(updateCheckInput(checkValues(loaded, 'ICMP'), edited)).toEqual({ name: 'renamed' });
  });

  it.each([
    ['domain', 'example.org'],
    ['warnDays', '14'],
    ['timeoutMs', '5000'],
  ] as const)('a changed %s sends the whole domainExpiry', (key, value) => {
    const initial = checkValues(loaded, 'ICMP');
    const edited = checkValues(loaded, 'ICMP');
    edited.domainExpiry[key] = value;
    const sent = { domain: 'example.net', warnDays: 30, timeoutMs: 10000 };
    const want = { ...sent, [key]: key === 'domain' ? value : Number(value) };
    expect(updateCheckInput(initial, edited)).toEqual({ config: { domainExpiry: want } });
  });

  it('CHK-009: an edit that changes only the warn days loads the stored domain, validates, and sends the whole config', () => {
    const initial = checkValues(loaded, 'ICMP');
    expect(initial.domainExpiry).toMatchObject({ domain: 'example.net', warnDays: '30', timeoutMs: '10000' });
    const edited = checkValues(loaded, 'ICMP');
    edited.domainExpiry.warnDays = '14';
    expect(validateCheck(edited, initial)).toEqual([]);
    expect(updateCheckInput(initial, edited)).toEqual({ config: { domainExpiry: { domain: 'example.net', warnDays: 14, timeoutMs: 10000 } } });
  });
});

const DOMAIN = { __typename: 'DomainExpiryConfig', domain: 'example.net', warnDays: 30, timeoutMs: 10_000 } as const;

describe('R-102, D-29, CHK-009: a domain expiry check takes no retries', () => {
  it('create sends retries 0 and no retry interval for a new check with defaults', () => {
    const values = { ...checkValues(null, 'DOMAIN_EXPIRY'), name: 'Registration' };
    values.domainExpiry.domain = 'example.net';
    const input = createCheckInput(values, 'dev-1');
    expect(input.retries).toBe(0);
    expect(input).not.toHaveProperty('retryIntervalSeconds');
  });

  it('create sends retries 0 when the hub default retries are 5', () => {
    const values = { ...checkValues(null, 'DOMAIN_EXPIRY'), name: 'Registration', retries: '5', retryIntervalSeconds: '45' };
    values.domainExpiry.domain = 'example.net';
    expect(createCheckInput(values, 'dev-1')).toMatchObject({ retries: 0 });
  });

  it('update sends retries 0 for a stored check that has retries above 0', () => {
    const initial = checkValues(saved('DOMAIN_EXPIRY', DOMAIN, { retries: 3 }), 'ICMP');
    expect(updateCheckInput(initial, { ...initial })).toEqual({ retries: 0 });
  });

  it('hidden retry fields that differ from the stored values are not sent (retries is sent as 0)', () => {
    const initial = checkValues(saved('DOMAIN_EXPIRY', DOMAIN, { retries: 3 }), 'ICMP');
    const edited = { ...initial, retries: '7', retryIntervalSeconds: '99', name: 'renamed' };
    expect(updateCheckInput(initial, edited)).toEqual({ name: 'renamed', retries: 0 });
  });

  it('update of a check already at 0 retries sends nothing', () => {
    const initial = checkValues(saved('DOMAIN_EXPIRY', DOMAIN), 'ICMP');
    expect(updateCheckInput(initial, { ...initial })).toEqual({});
  });

  it('a hidden retries value does not block the save', () => {
    const values = { ...checkValues(null, 'DOMAIN_EXPIRY'), name: 'Registration', retries: '99' };
    values.domainExpiry.domain = 'example.net';
    expect(validateCheck(values, null).map((error) => error.field)).not.toContain('retries');
  });

  it('another type (TCP) still sends its retries and retry interval', () => {
    const values = { ...checkValues(null, 'TCP'), retries: '5', retryIntervalSeconds: '30' };
    expect(createCheckInput(values, 'dev-1')).toMatchObject({ retries: 5, retryIntervalSeconds: 30 });
    const initial = checkValues(saved('TCP', { __typename: 'TcpConfig', port: 22, timeoutMs: 3000 }), 'ICMP');
    expect(updateCheckInput(initial, { ...initial, retries: '4' })).toEqual({ retries: 4 });
  });
});
