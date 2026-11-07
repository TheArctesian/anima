import { describe, expect, it } from 'vitest';
import { check } from '../page.fixture.svelte';
import { RESPONSE_POINTS, responseSteps, stepsAfter } from './response';
import { checkRowText } from './rows';

describe('checkRowText (D-10, CHK-007)', () => {
  it('D-10: type, last response time, and no certificate for a ping', () => {
    expect(checkRowText(check())).toEqual({ type: 'Ping', responseTime: '12 ms', certificate: null });
  });

  it('CHK-007: certificate days for a check that reads one', () => {
    expect(checkRowText(check({ type: 'HTTP', certificateDaysLeft: 30 })).certificate).toBe('Certificate: 30 days');
    expect(checkRowText(check({ type: 'TLS', certificateDaysLeft: 1 })).certificate).toBe('Certificate: 1 day');
    expect(checkRowText(check({ type: 'TLS', certificateDaysLeft: 0 })).certificate).toBe('Certificate: 0 days');
  });

  it('D-10: no heartbeat yet, or one without a time, is "—"', () => {
    expect(checkRowText(check({ lastHeartbeat: null })).responseTime).toBe('—');
    expect(checkRowText(check({ lastHeartbeat: { at: '2026-11-04T10:00:00Z', status: 'DOWN', responseTimeMs: null, message: 'x' } })).responseTime).toBe('—');
  });
});

describe('responseSteps (D-10, Chart update role)', () => {
  const beat = (minute: number, ms: number | null) => ({ at: `2026-11-04T10:${String(minute).padStart(2, '0')}:00Z`, responseTimeMs: ms });

  it('D-10: the newest beat is at the newest step, the right edge of the window, one step per beat to the left', () => {
    const drawn = responseSteps([beat(2, 50), beat(1, null)], 7);
    expect(drawn.points).toEqual([{ at: 7, value: 50 }, { at: 6, value: null }]);
    expect(drawn.span).toEqual({ start: 7 - (RESPONSE_POINTS - 1), end: 7 });
  });

  it('D-10: one page of beats is in the window, and one beat more just left of it for the slide', () => {
    const many = Array.from({ length: RESPONSE_POINTS + 20 }, (_, index) => beat(index % 60, 10));
    const { points, span } = responseSteps(many, 0);
    expect(points.filter((point) => point.at >= span.start)).toHaveLength(RESPONSE_POINTS);
    expect(points.map((point) => point.at).filter((at) => at < span.start)).toEqual([span.start - 1]);
  });

  it('Chart update: each newer beat is one step more; a first load or older beats are none', () => {
    expect(stepsAfter([beat(1, 10)], [beat(3, 10), beat(2, 10), beat(1, 10)], 4)).toBe(6);
    expect(stepsAfter([beat(2, 10)], [beat(2, 10), beat(1, 10)], 4)).toBe(4);
    expect(stepsAfter([], [beat(1, 10)], 4)).toBe(4);
  });
});
