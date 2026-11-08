import { describe, expect, it } from 'vitest';
import { fieldsOf, fresh, saved } from '../check.fixture';
import { configInput } from '../model/config-input';
import { checkValues } from '../model/values';
import { daysLeftText, intervalFor } from './domain-expiry';

const GOOD = (values: ReturnType<typeof fresh>) => {
  values.domainExpiry.domain = 'example.net';
  values.intervalSeconds = '43200';
};

describe('CHK-009, D-29: DOMAIN_EXPIRY values', () => {
  it('a new check has warn days 30, timeout 10000 ms and no domain', () => {
    expect(checkValues(null, 'DOMAIN_EXPIRY').domainExpiry).toEqual({ domain: '', warnDays: '30', timeoutMs: '10000', daysLeft: null });
  });

  it('sends config: { domainExpiry: { domain, warnDays, timeoutMs } }', () => {
    const values = fresh('DOMAIN_EXPIRY', (v) => {
      v.domainExpiry = { domain: ' example.net ', warnDays: '14', timeoutMs: '5000', daysLeft: 9 };
    });
    expect(configInput(values)).toEqual({ domainExpiry: { domain: 'example.net', warnDays: 14, timeoutMs: 5000 } });
  });

  it('an existing check loads its own domain, warn days, timeout and days left', () => {
    const check = saved('DOMAIN_EXPIRY', { __typename: 'DomainExpiryConfig', domain: 'example.net', warnDays: 14, timeoutMs: 5000 }, { domainDaysLeft: 9 });
    expect(checkValues(check, 'ICMP').domainExpiry).toEqual({ domain: 'example.net', warnDays: '14', timeoutMs: '5000', daysLeft: 9 });
  });
});

describe('CHK-009, D-29: DOMAIN_EXPIRY validation', () => {
  it('accepts a domain at 12 hours', () => {
    expect(fieldsOf(fresh('DOMAIN_EXPIRY', GOOD))).toEqual([]);
  });

  it('accepts exactly 1 hour and refuses 3599 seconds with a fixed sentence', () => {
    expect(fieldsOf(fresh('DOMAIN_EXPIRY', (v) => { GOOD(v); v.intervalSeconds = '3600'; }))).toEqual([]);
    const refused = fresh('DOMAIN_EXPIRY', (v) => { GOOD(v); v.intervalSeconds = '3599'; });
    expect(fieldsOf(refused)).toEqual(['intervalSeconds']);
  });

  it('refuses an empty domain, an address, bad warn days and a bad timeout', () => {
    expect(fieldsOf(fresh('DOMAIN_EXPIRY', (v) => { GOOD(v); v.domainExpiry.domain = ''; }))).toEqual(['domain']);
    expect(fieldsOf(fresh('DOMAIN_EXPIRY', (v) => { GOOD(v); v.domainExpiry.domain = '192.0.2.1'; }))).toEqual(['domain']);
    expect(fieldsOf(fresh('DOMAIN_EXPIRY', (v) => { GOOD(v); v.domainExpiry.warnDays = '0'; }))).toEqual(['warnDays']);
    expect(fieldsOf(fresh('DOMAIN_EXPIRY', (v) => { GOOD(v); v.domainExpiry.timeoutMs = '5'; }))).toEqual(['timeoutMs']);
  });
});

describe('D-29: the proposed interval', () => {
  it('is 12 hours for an untouched interval, and the ordinary one for other types', () => {
    expect(intervalFor('DOMAIN_EXPIRY', '60', '60', false)).toBe('43200');
    expect(intervalFor('TCP', '43200', '60', false)).toBe('60');
  });

  it('keeps a touched interval', () => {
    expect(intervalFor('DOMAIN_EXPIRY', '7200', '60', true)).toBe('7200');
  });
});

describe('CHK-009: days left', () => {
  it('reads in days, one day, and not known', () => {
    expect(daysLeftText(9)).toBe('Days left: 9 days');
    expect(daysLeftText(1)).toBe('Days left: 1 day');
    expect(daysLeftText(null)).toBe('Days left: not known yet');
  });
});
