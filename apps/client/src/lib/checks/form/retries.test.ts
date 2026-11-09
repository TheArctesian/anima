import { describe, expect, it } from 'vitest';
import { saved } from './check.fixture';
import { createCheckInput, updateCheckInput } from './model/input';
import { validateCheck } from './model/validate';
import { checkValues } from './model/values';

const DOMAIN = { __typename: 'DomainExpiryConfig', domain: 'example.net', warnDays: 30, timeoutMs: 10_000 } as const;

describe('R-102, D-29, CHK-009: a domain expiry check takes no retries', () => {
  it('create sends retries 0 and no retry interval for a new check with defaults', () => {
    const values = { ...checkValues(null, 'DOMAIN_EXPIRY'), name: 'Registration' };
    values.domainExpiry.domain = 'example.net';
    const input = createCheckInput(values, 'dev-1');
    expect(input.retries).toBe(0);
    expect(input).not.toHaveProperty('retryIntervalSeconds');
  });

  it('update sends retries 0 for a stored check that has retries above 0', () => {
    const initial = checkValues(saved('DOMAIN_EXPIRY', DOMAIN, { retries: 3 }), 'ICMP');
    expect(updateCheckInput(initial, { ...initial })).toEqual({ retries: 0 });
  });

  it('update sends retries 0 even when the defaults patch changed the hidden retry fields', () => {
    const initial = checkValues(saved('DOMAIN_EXPIRY', DOMAIN, { retries: 3 }), 'ICMP');
    const edited = { ...initial, retries: '7', retryIntervalSeconds: '99', name: 'renamed' };
    expect(updateCheckInput(initial, edited)).toEqual({ name: 'renamed', retries: 0 });
  });

  it('update of a check already at 0 retries sends nothing', () => {
    const initial = checkValues(saved('DOMAIN_EXPIRY', DOMAIN, { retries: 0 }), 'ICMP');
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
