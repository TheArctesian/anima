import { expect, it } from 'vitest';
import { session } from '../page.fixture.svelte';
import { durationText, sessionRows } from './sessions';

const NOW = new Date('2026-11-04T12:00:00Z');

it('D-10: sessions are newest first, with end, duration, operator and end reason', () => {
  const rows = sessionRows(
    [
      session('old', '2026-11-04T10:00:00Z', { endedAt: '2026-11-04T11:05:00Z', endReason: 'REMOTE_CLOSED' }),
      session('new', '2026-11-04T11:58:00Z'),
    ],
    NOW,
  );
  expect(rows.map((row) => row.id)).toEqual(['new', 'old']);
  expect(rows[0]).toMatchObject({ end: 'Live', duration: '2 min 0 s', operator: 'ada', reason: '—' });
  expect(rows[1]).toMatchObject({ duration: '1 h 5 min', reason: 'Closed by the device' });
});

it.each([
  [0, '0 s'],
  [59.9, '59 s'],
  [61, '1 min 1 s'],
  [3600, '1 h 0 min'],
  [-5, '0 s'],
])('D-10: %d seconds is %j', (seconds, text) => {
  expect(durationText(seconds)).toBe(text);
});

it('D-10: a session with a time that does not parse has no duration', () => {
  expect(sessionRows([session('x', 'not a time')], NOW)[0]?.duration).toBe('—');
});
