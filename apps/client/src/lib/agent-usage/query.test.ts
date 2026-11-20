import { describe, expect, it } from 'vitest';
import { DEFAULT_QUERY, parseUsageQuery, usageRequest, writeUsageQuery, type UsageQuery } from './query';

const read = (search: string): UsageQuery => parseUsageQuery(new URLSearchParams(search));

describe('the address of Agent usage', () => {
  it('an address with no choice is the default: 30 days, by cost, every device, program and repository', () => {
    expect(read('')).toEqual(DEFAULT_QUERY);
    expect(writeUsageQuery(DEFAULT_QUERY).toString()).toBe('');
  });

  it('each choice goes to the address in a fixed order and comes back from it', () => {
    const query: UsageQuery = { days: 90, count: 'tokens', device: 'd-mac', source: 'CODEX', repository: 'github.com/acme/anima' };
    const search = writeUsageQuery(query).toString();
    expect(search).toBe('days=90&count=tokens&device=d-mac&source=CODEX&repository=github.com%2Facme%2Fanima');
    expect(read(search)).toEqual(query);
  });

  it('a value that the page does not know falls back to the default, since the address is typed by hand', () => {
    expect(read('days=45&count=prompts&source=COPILOT&device=&repository=')).toEqual(DEFAULT_QUERY);
  });
});

describe('what the hub is asked (AIS-020)', () => {
  const NOW = new Date('2026-11-17T23:30:00Z');

  it('the days end today in UTC and include today', () => {
    expect(usageRequest(DEFAULT_QUERY, NOW).filter).toEqual({ firstDay: '2026-10-19', lastDay: '2026-11-17' });
    expect(usageRequest({ ...DEFAULT_QUERY, days: 7 }, NOW).filter).toEqual({ firstDay: '2026-11-11', lastDay: '2026-11-17' });
  });

  it('a device, a program and a repository narrow the request; what is counted does not change it', () => {
    const query: UsageQuery = { days: 7, count: 'sessions', device: 'd-mac', source: 'CLAUDE_CODE', repository: 'github.com/acme/anima' };
    expect(usageRequest(query, NOW).filter).toEqual({
      firstDay: '2026-11-11',
      lastDay: '2026-11-17',
      deviceId: 'd-mac',
      source: 'CLAUDE_CODE',
      repository: 'github.com/acme/anima',
    });
    expect(usageRequest({ ...query, count: 'cost' }, NOW)).toEqual(usageRequest(query, NOW));
  });
});
