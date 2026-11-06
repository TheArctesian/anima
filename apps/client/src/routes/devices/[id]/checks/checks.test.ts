import { describe, expect, it } from 'vitest';
import { check } from '../page.fixture.svelte';
import { CHART_BOX, CHART_POINTS, CHART_STEP, chartLines, gainedNewest } from './chart';
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

describe('chartLines (D-10, Chart update role)', () => {
  const beat = (minute: number, ms: number | null) => ({ at: `2026-11-04T10:${String(minute).padStart(2, '0')}:00Z`, responseTimeMs: ms });

  it('D-10: the newest beat is at the right edge and the slowest at the top', () => {
    const lines = chartLines([beat(2, 50), beat(1, 100)]);
    expect(lines.maxMs).toBe(100);
    const right = String(CHART_BOX.width);
    const left = String(Math.round((CHART_BOX.width - CHART_STEP) * 100) / 100);
    expect(lines.segments).toEqual([`${left},0 ${right},${String(CHART_BOX.height / 2)}`]);
  });

  it('D-10: a beat without a response time breaks the line', () => {
    expect(chartLines([beat(3, 10), beat(2, null), beat(1, 10)]).segments).toHaveLength(2);
  });

  it('D-10: at most one page of beats is drawn', () => {
    const many = Array.from({ length: CHART_POINTS + 20 }, (_, index) => beat(index % 60, 10));
    expect(chartLines(many).segments[0]?.split(' ')).toHaveLength(CHART_POINTS);
  });

  it('Chart update: only a newer newest beat moves the line', () => {
    expect(gainedNewest([beat(1, 10)], [beat(2, 10), beat(1, 10)])).toBe(true);
    expect(gainedNewest([beat(2, 10)], [beat(2, 10), beat(1, 10)])).toBe(false);
    expect(gainedNewest([], [beat(1, 10)])).toBe(false);
  });
});
