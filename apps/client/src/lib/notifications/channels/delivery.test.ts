import { describe, expect, it } from 'vitest';
import { DELIVERY_REASONS } from '$lib/fixtures/phase3';
import type { Delivery } from '../api';
import { NO_REASON, reasonOf, testOutcome, UNKNOWN_REASON } from './delivery';

function failed(reason: string | null, code: number | null = null): Delivery {
  return { id: 'd', channelId: 'c', kind: 'TEST', at: '2026-11-07T12:00:00Z', ok: false, reason: reason as Delivery['reason'], code };
}

describe('reasonOf', () => {
  it('CR-14.5, NOT-004: every K1 reason has its own fixed sentence', () => {
    const sentences = DELIVERY_REASONS.map((reason) => reasonOf(failed(reason)));
    expect(sentences.every((sentence) => sentence !== '' && sentence !== UNKNOWN_REASON && sentence !== NO_REASON)).toBe(true);
    expect(new Set(sentences).size).toBe(DELIVERY_REASONS.length);
    expect(reasonOf(failed('TIMED_OUT'))).toBe('No answer in 8 seconds.');
  });

  it('MOT-007: every sentence, with any code shown, fits one line of a row at 390 px (29 characters)', () => {
    const all = [...DELIVERY_REASONS, 'NEW_REASON', null].flatMap((reason) => [reasonOf(failed(reason)), reasonOf(failed(reason, 599))]);
    expect(all.filter((sentence) => sentence.length > 29)).toEqual([]);
  });

  it('CR-14.5: a delivery that went through has no reason line', () => {
    expect(reasonOf({ ...failed(null), ok: true })).toBe('');
    expect(testOutcome({ ...failed(null), ok: true })).toBe('Test delivered.');
  });

  it('CR-14.5: the code is shown as a number for HTTP_STATUS and SMTP_REFUSED only, and only in its range', () => {
    expect(reasonOf(failed('HTTP_STATUS', 503))).toBe('Target answered HTTP 503.');
    expect(reasonOf(failed('SMTP_REFUSED', 550))).toBe('Mail server refused (550).');
    expect(reasonOf(failed('HTTP_STATUS', 99_999))).toBe('Target answered an error.');
    expect(reasonOf(failed('HTTP_STATUS', 503.5))).toBe('Target answered an error.');
    expect(reasonOf(failed('TIMED_OUT', 503))).toBe('No answer in 8 seconds.');
  });

  it.each([
    ['HTTP_STATUS', 99, false],
    ['HTTP_STATUS', 100, true],
    ['HTTP_STATUS', 599, true],
    ['HTTP_STATUS', 600, false],
    ['SMTP_REFUSED', 5, false],
    ['SMTP_REFUSED', 199, false],
    ['SMTP_REFUSED', 200, true],
    ['SMTP_REFUSED', 599, true],
    ['SMTP_REFUSED', 600, false],
  ])('CR-14.5: %s with code %i shows the code: %s', (reason, code, shown) => {
    expect(reasonOf(failed(reason, code)).includes(String(code))).toBe(shown);
  });

  it('CR-14.5, LOG-007: a value this client does not know gets a fixed fallback, never the value itself', () => {
    for (const reason of ['TARGET_SAID_HELLO', 'constructor', '__proto__', 'toString']) expect(reasonOf(failed(reason))).toBe(UNKNOWN_REASON);
    expect(reasonOf(failed(null))).toBe(NO_REASON);
    expect(testOutcome(failed('TARGET_NOT_PERMITTED'))).toBe('Test failed. Target address not permitted.');
  });
});
