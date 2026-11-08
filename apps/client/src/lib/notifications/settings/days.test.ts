import { describe, expect, it } from 'vitest';
import { checkDays, DAYS_MISSING, DAYS_NOT_DESCENDING, DAYS_NOT_WHOLE, DAYS_OUT_OF_RANGE, DAYS_TOO_MANY, daysText, DEFAULT_DAYS } from './days';

describe('certificate expiry days', () => {
  it('NOT-006, D-22: the default is 21, 14 and 7, and it reads back as typed', () => {
    expect(DEFAULT_DAYS).toEqual([21, 14, 7]);
    expect(checkDays(daysText(DEFAULT_DAYS))).toEqual({ ok: true, days: [21, 14, 7] });
    expect(checkDays(' 30 21,14 , 7 ')).toEqual({ ok: true, days: [30, 21, 14, 7] });
  });

  it('D-22: each value is from 1 to 365', () => {
    expect(checkDays('365, 1')).toEqual({ ok: true, days: [365, 1] });
    expect(checkDays('366, 14')).toEqual({ ok: false, message: DAYS_OUT_OF_RANGE });
    expect(checkDays('14, 0')).toEqual({ ok: false, message: DAYS_OUT_OF_RANGE });
  });

  it('D-22: at most five values', () => {
    expect(checkDays('60, 30, 21, 14, 7')).toEqual({ ok: true, days: [60, 30, 21, 14, 7] });
    expect(checkDays('90, 60, 30, 21, 14, 7')).toEqual({ ok: false, message: DAYS_TOO_MANY });
  });

  it('D-22: the values descend, each once', () => {
    expect(checkDays('7, 14, 21')).toEqual({ ok: false, message: DAYS_NOT_DESCENDING });
    expect(checkDays('21, 14, 14')).toEqual({ ok: false, message: DAYS_NOT_DESCENDING });
  });

  it('D-22: only whole numbers, and at least one', () => {
    for (const text of ['21.5', '-7', '14 days', '1e2', '0x10']) expect(checkDays(text)).toEqual({ ok: false, message: DAYS_NOT_WHOLE });
    expect(checkDays(' , ')).toEqual({ ok: false, message: DAYS_MISSING });
  });

  it('MOT-007: each refusal fits the one line kept under the field at 390 px (36 characters)', () => {
    const messages = [DAYS_MISSING, DAYS_NOT_DESCENDING, DAYS_NOT_WHOLE, DAYS_OUT_OF_RANGE, DAYS_TOO_MANY];
    expect(messages.filter((message) => message.length > 36)).toEqual([]);
  });
});
