import { describe, expect, it } from 'vitest';
import { POOL_ANSWER, withHub } from './usage.fixture';
import { amountText, figuresTime, hoursText, hubDataBytes, parseUsage, planDiskText } from './usage';


describe('the figures of the account (HMD-073, contracts/account-usage.md)', () => {
  it('takes the answer of the pool in the form of the contract', () => {
    const usage = parseUsage(POOL_ANSWER);
    expect(usage?.at.toISOString()).toBe('2026-11-20T08:00:00.000Z');
    expect([usage?.month, usage?.plan, usage?.awakeSeconds, usage?.asleepSeconds]).toEqual(['2026-11', { name: 'Hobby', charged: true }, 45_000, 1_800]);
    expect(usage?.amount).toEqual({ minor: 412, currency: 'USD' });
    expect(usage?.disk).toEqual({ limitBytes: 10_000_000_000, usedBytes: 2_500_000_000 });
    expect(parseUsage(withHub({ plan: 'free' }))?.plan).toEqual({ name: 'Free', charged: false });
    expect(parseUsage(withHub({ plan: 'free' }))?.amount).toBeNull();
    expect(parseUsage({ ...POOL_ANSWER, prices_set: false })?.amount).toBeNull();
    expect(parseUsage(withHub({ disk_used_bytes: null }))?.disk.usedBytes).toBeNull();
  });

  it('takes the hub of the page: the one it names, else the only one of the account', () => {
    const two = { ...POOL_ANSWER, hubs: [POOL_ANSWER.hubs[0], { ...POOL_ANSWER.hubs[0], id: 'h2', plan: 'free' }] };
    expect(parseUsage(two, 'h2')?.plan.name).toBe('Free');
    expect(parseUsage(two)).toBeUndefined();
    expect(parseUsage(POOL_ANSWER, 'hub')?.plan.name).toBe('Hobby');
  });

  it('refuses an answer with a member missing or of the wrong kind', () => {
    for (const wrong of [null, {}, { ...POOL_ANSWER, until: 'now' }, { ...POOL_ANSWER, period: '11/2026' }, { ...POOL_ANSWER, total: 1.5 },
      { ...POOL_ANSWER, currency: 'dollars' }, { ...POOL_ANSWER, lines: [{ kind: 'running', seconds: -1 }] }, { ...POOL_ANSWER, hubs: [] },
      { ...POOL_ANSWER, prices_set: 'yes' }, withHub({ disk_limit_bytes: undefined }), withHub({ plan: '' })]) {
      expect(parseUsage(wrong)).toBeUndefined();
    }
  });

  it('says the hours, the amount in its currency and the disk of the plan as one figure of its own', () => {
    expect([hoursText(1_800), hoursText(45_000)]).toEqual(['0.5 h', '12 h']);
    expect(amountText({ minor: 412, currency: 'USD' }, 'en-US')).toBe('$4.12');
    expect(amountText({ minor: 412, currency: 'JPY' }, 'en-US')).toBe('¥412');
    expect(planDiskText({ limitBytes: 10_000_000_000, usedBytes: 2_500_000_000 })).toMatch(/ of .*\(25%\)$/);
    expect(planDiskText({ limitBytes: 10_000_000_000, usedBytes: null })).toMatch(/^Limit /);
  });

  it('says how old the figures are', () => {
    const at = new Date('2026-11-20T18:00:00Z');
    expect(figuresTime(at, new Date('2026-11-20T18:00:30Z'))).toBe('Figures from just now');
    expect(figuresTime(at, new Date('2026-11-20T18:12:00Z'))).toBe('Figures from 12 min ago');
    expect(figuresTime(at, new Date('2026-11-20T20:00:00Z'))).toMatch(/^Figures from .*2026/);
  });

  it('HMD-072: the data of the hub is the newest DISK_BYTES value of its own device', () => {
    const points = (values: (number | null)[]) => values.map((value) => ({ value }));
    expect(hubDataBytes([{ kind: 'CPU_PERCENT', points: points([5]) }, { kind: 'DISK_BYTES', points: points([10, 20, null]) }])).toBe(20);
    expect(hubDataBytes([])).toBeNull();
  });
});
