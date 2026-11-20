import { describe, expect, it } from 'vitest';
import { costText, estimateNote, leftOutLine, sourceName, stateWords } from './words';

describe('the words of Agent usage', () => {
  it('AIS-020: a program has its name, and one that this client does not know shows as the hub names it', () => {
    expect(sourceName('CLAUDE_CODE')).toBe('Claude Code');
    expect(sourceName('CODEX')).toBe('Codex');
    expect(sourceName('NEW_ONE' as never)).toBe('NEW_ONE');
    expect(sourceName(null)).toBe('No program');
  });

  it('AIS-024: each state of a daemon has its words, and only the reading one is the working state', () => {
    expect(stateWords('READING')).toEqual({ words: 'Reads sessions', reads: true });
    expect(stateWords('SYSTEM_UNIT')).toEqual({ words: "Runs as a system unit: it cannot see a user's sessions", reads: false });
    expect(stateWords('TURNED_OFF')).toEqual({ words: "Turned off in the daemon's settings", reads: false });
  });

  it('AIS-021: a cost is dollars with cents, and a group with no price says so', () => {
    expect(costText({ estimatedCostUsd: 41.2 })).toBe('$41.20');
    expect(costText({ estimatedCostUsd: 0.003 })).toBe('$<0.01');
    expect(costText({ estimatedCostUsd: null })).toBe('No price');
  });

  it('AIS-021: the estimate says that it is not a bill, and names the tokens with no price only when there are some', () => {
    expect(estimateNote('2026-11-01', 0)).toBe('list prices of 2026-11-01; not a bill');
    expect(estimateNote('2026-11-01', 1_200_000)).toBe('list prices of 2026-11-01; not a bill. 1.2M tokens have no known price');
  });

  it('the groups that the hub left out are counted in words, and none gives no line', () => {
    expect(leftOutLine(0)).toBe('');
    expect(leftOutLine(1)).toBe('1 more group is not shown. The totals count them.');
    expect(leftOutLine(12)).toBe('12 more groups are not shown. The totals count them.');
  });
});
