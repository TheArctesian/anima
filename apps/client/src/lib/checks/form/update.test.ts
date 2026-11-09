import { describe, expect, it } from 'vitest';
import { saved } from './check.fixture';
import { updateCheckInput } from './model/input';
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
