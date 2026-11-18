import { describe, expect, it } from 'vitest';
import { DEFAULT_QUERY, parseUsageQuery, usageRequest, withGroup, writeUsageQuery } from './query';

const params = (search: string) => new URLSearchParams(search);

describe('the choices of the usage page in the address (AIS-020)', () => {
  it('an empty address is the default: 30 days, by device and program', () => {
    expect(parseUsageQuery(params(''))).toEqual(DEFAULT_QUERY);
    expect(writeUsageQuery(DEFAULT_QUERY).toString()).toBe('');
  });

  it('every choice survives a round trip through the address', () => {
    const query = { days: 90, group: ['DAY', 'MODEL'], device: 'd-mac', source: 'CODEX' } as const;
    const search = writeUsageQuery(query).toString();
    expect(search).toBe('days=90&by=DAY&by=MODEL&device=d-mac&source=CODEX');
    expect(parseUsageQuery(params(search))).toEqual(query);
  });

  it('a value typed by hand that the page does not know falls back to the default', () => {
    expect(parseUsageQuery(params('days=45&by=COLOUR&source=gemini'))).toEqual(DEFAULT_QUERY);
  });

  it('the groups keep the order of the table whatever order the address has', () => {
    expect(parseUsageQuery(params('by=REPOSITORY&by=DEVICE&by=DEVICE')).group).toEqual(['DEVICE', 'REPOSITORY']);
  });

  it('a group goes on and off, and the last one stays on', () => {
    expect(withGroup(['DEVICE', 'SOURCE'], 'MODEL')).toEqual(['DEVICE', 'SOURCE', 'MODEL']);
    expect(withGroup(['DEVICE', 'SOURCE'], 'DEVICE')).toEqual(['SOURCE']);
    expect(withGroup(['SOURCE'], 'SOURCE')).toEqual(['SOURCE']);
  });
});

describe('what the page asks the hub for', () => {
  const now = new Date('2026-11-17T23:30:00Z');

  it('the days end today in UTC and the first day is days - 1 before', () => {
    expect(usageRequest(DEFAULT_QUERY, now).filter).toEqual({ firstDay: '2026-10-19', lastDay: '2026-11-17' });
    expect(usageRequest({ ...DEFAULT_QUERY, days: 7 }, now).filter).toEqual({ firstDay: '2026-11-11', lastDay: '2026-11-17' });
  });

  it('the device and the program narrow the use, and the group is the sum', () => {
    const request = usageRequest({ days: 7, group: ['MODEL'], device: 'd-mac', source: 'CLAUDE_CODE' }, now);
    expect(request.filter).toMatchObject({ deviceId: 'd-mac', source: 'CLAUDE_CODE' });
    expect(request.by).toEqual(['MODEL']);
  });
});
