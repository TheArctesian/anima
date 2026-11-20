import { describe, expect, it } from 'vitest';
import { ANSWER, sums } from '../agent-usage.fixture';
import { countOf, dayBars, lastUseWords, repositoryLines, share, shortDay, tokenParts, totalTokens } from './shape';

describe('what Agent usage draws from the sums of the hub (AIS-020)', () => {
  it('there is one bar for each UTC day of the view, a day with no use included, and the tallest day is the whole height', () => {
    const bars = dayBars(ANSWER, 'cost', null);
    expect(bars.map((bar) => bar.day)).toEqual(['2026-11-11', '2026-11-12', '2026-11-13', '2026-11-14', '2026-11-15', '2026-11-16', '2026-11-17']);
    expect(bars.map((bar) => bar.value)).toEqual([0, 0, 10, 0, 0, 0, 5]);
    expect(bars.map((bar) => bar.height)).toEqual([0, 0, 1, 0, 0, 0, 0.5]);
  });

  it('the bars count what the page counts, and only the rows of a picked program and model', () => {
    expect(dayBars(ANSWER, 'sessions', null).at(-1)?.value).toBe(7);
    expect(dayBars(ANSWER, 'tokens', null).at(-1)?.value).toBe(1500);
    const codex = dayBars(ANSWER, 'cost', { source: 'CODEX', model: 'gpt-5-codex' });
    expect(codex.map((bar) => bar.value)).toEqual([0, 0, 0, 0, 0, 0, 1]);
    expect(codex.at(-1)?.height).toBe(1);
  });

  it('a view with no use has bars of no height, and days that are not days give no bar', () => {
    expect(dayBars({ ...ANSWER, days: [] }, 'cost', null).every((bar) => bar.height === 0)).toBe(true);
    expect(dayBars({ firstDay: 'x', lastDay: 'y', days: [] }, 'cost', null)).toEqual([]);
    expect(dayBars({ firstDay: '2026-11-17', lastDay: '2026-11-11', days: [] }, 'cost', null)).toEqual([]);
  });

  it('the repositories stand newest use first, and the use with no repository is last', () => {
    expect(repositoryLines(ANSWER).map((line) => [line.repository, line.lastDay])).toEqual([
      ['github.com/acme/anima', '2026-11-17'],
      ['github.com/acme/blog', '2026-11-14'],
      [null, '2026-11-17'],
    ]);
  });

  it('with the same newest day, or none known, the larger estimate is first', () => {
    const lines = repositoryLines({
      repositories: [
        { ...sums({ estimatedCostUsd: 1 }), repository: 'a' },
        { ...sums({ estimatedCostUsd: 5 }), repository: 'b' },
        { ...sums({ estimatedCostUsd: 3 }), repository: 'c' },
      ],
      newest: [{ repository: 'a', day: '2026-11-17' }],
    });
    expect(lines.map((line) => line.repository)).toEqual(['a', 'b', 'c']);
  });

  it('the last use is said in words from two UTC days', () => {
    expect(lastUseWords('2026-11-17', '2026-11-17')).toBe('last use today');
    expect(lastUseWords('2026-11-16', '2026-11-17')).toBe('last use yesterday');
    expect(lastUseWords('2026-11-14', '2026-11-17')).toBe('last use 3 days ago');
    expect(lastUseWords(null, '2026-11-17')).toBe('');
    expect(lastUseWords('2026-11-18', '2026-11-17')).toBe('');
  });

  it('a count, a share and the four kinds of tokens come from the sums; a group with no price counts no cost', () => {
    const group = sums({ sessions: 3, inputTokens: 1, outputTokens: 2, cacheReadTokens: 3, cacheWriteTokens: 4, estimatedCostUsd: null });
    expect(totalTokens(group)).toBe(10);
    expect([countOf(group, 'cost'), countOf(group, 'tokens'), countOf(group, 'sessions')]).toEqual([0, 10, 3]);
    expect(tokenParts(group).map((part) => [part.label, part.tokens])).toEqual([['Input', 1], ['Output', 2], ['Cache read', 3], ['Cache write', 4]]);
    expect([share(1, 4), share(5, 4), share(1, 0), share(-1, 4)]).toEqual([0.25, 1, 0, 0]);
  });

  it('a day at an end of the chart is short', () => {
    expect(shortDay('2026-11-01')).toBe('1 Nov');
    expect(shortDay('not a day')).toBe('not a day');
  });
});
