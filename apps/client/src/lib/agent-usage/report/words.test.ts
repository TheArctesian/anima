import { describe, expect, it } from 'vitest';
import { ANSWER, DEVICES, row } from '../agent-usage.fixture';
import { dollars, tokens } from './units';
import { SYSTEM_UNIT, TURNED_OFF, deviceNotes, notes, summaryLine } from './words';

describe('the units are those of `anima agents usage`', () => {
  it.each([
    [950, '950'],
    [18_300, '18.3k'],
    [2_200_000, '2.2M'],
    [1_400_000_000, '1.4B'],
  ])('%d tokens read %s', (count, text) => {
    expect(tokens(count)).toBe(text);
  });

  it('a cost above zero that rounds to none reads <0.01', () => {
    expect(dollars(0.003)).toBe('<0.01');
    expect(dollars(12.4)).toBe('12.40');
    expect(dollars(0)).toBe('0.00');
  });
});

describe('the summary line (AIS-020, AIS-021)', () => {
  it('gives sessions, all tokens and the estimate', () => {
    const total = row({ sessions: 17, inputTokens: 1000, outputTokens: 2000, cacheReadTokens: 1_000_000, cacheWriteTokens: 5000, estimatedCostUsd: 12.4 });
    expect(summaryLine(total)).toBe('17 sessions, 1.0M tokens, estimated cost $12.40');
  });

  it('says one session in the singular, and no price when no model has one', () => {
    expect(summaryLine(row({ sessions: 1, inputTokens: 10, estimatedCostUsd: null }))).toBe('1 session, 10 tokens, no estimated cost');
  });
});

describe('the notes under the table', () => {
  it('AIS-021: the estimate is not the bill of a subscription', () => {
    expect(notes({ ...ANSWER, total: row({}), rowsLeftOut: 0 })).toEqual([
      'The cost is an estimate from the public list prices of 2026-11-01. It is not the bill of a subscription.',
    ]);
  });

  it('AIS-021: the tokens with no price are counted when there are some', () => {
    expect(notes(ANSWER)).toContain('1.4B tokens have no price and are not in the estimate.');
  });

  it('the rows the hub left out are counted when there are some', () => {
    expect(notes({ ...ANSWER, rowsLeftOut: 12 })).toContain('12 rows are not shown.');
    expect(notes({ ...ANSWER, rowsLeftOut: 1 })).toContain('1 row is not shown.');
    expect(notes(ANSWER).join(' ')).not.toContain('not shown');
  });

  it('AIS-024: a system unit and a device turned off each give their reason; a device that reads gives none', () => {
    expect(deviceNotes(DEVICES)).toEqual([
      { id: 'd-box', name: 'gpu-box', sentence: TURNED_OFF },
      { id: 'd-nas', name: 'nas', sentence: SYSTEM_UNIT },
    ]);
    expect(SYSTEM_UNIT).toBe('This daemon runs as a system unit and cannot see the sessions of a user.');
  });
});
