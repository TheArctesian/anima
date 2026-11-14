import { expect, it } from 'vitest';
import type { SourceHealth } from './api';
import { agoText, healthWords, lastSyncText } from './health';
import { runLine } from './runs/runs';
import { run } from './sources.fixture';

const STATES: readonly SourceHealth[] = ['OK', 'STARTING', 'STALLED', 'NEVER_SYNCED', 'PAUSED', 'NEEDS_RECONNECT'];

it('SRC-009: each of the six health states has its own name and its own sentence', () => {
  const words = STATES.map((health) => healthWords({ health, pausedBy: health === 'PAUSED' ? 'OPERATOR' : null }));
  expect(words.map((each) => each.name)).toEqual(['OK', 'Starting', 'Stalled', 'Never synced', 'Paused', 'Needs reconnect']);
  expect(new Set(words.map((each) => each.meaning)).size).toBe(6);
});

it('SRC-009: Paused says who paused it', () => {
  expect(healthWords({ health: 'PAUSED', pausedBy: 'OPERATOR' }).meaning).toContain('You paused it');
  expect(healthWords({ health: 'PAUSED', pausedBy: 'EXTENSION_OFF' }).meaning).toContain('extension');
});

it('SRC-003: the last sync reads as "Synced 4 min ago", or "Never synced"', () => {
  const now = new Date('2026-11-13T12:00:00Z');
  expect(lastSyncText('2026-11-13T11:56:00Z', now)).toBe('Synced 4 min ago');
  expect(lastSyncText(null, now)).toBe('Never synced');
  expect(agoText('2026-11-10T12:00:00Z', now)).toBe('3 days ago');
});

it('SRC-005: a run line carries the trigger and the four counts', () => {
  const line = runLine(run('r', { trigger: 'MANUAL', inserted: 3, changed: 2, unchanged: 9, removed: 1 }), new Date('2026-11-13T12:00:00Z'));
  expect(line.trigger).toBe('By hand');
  expect(line.counts).toEqual([
    { label: 'Inserted', value: 3 },
    { label: 'Changed', value: 2 },
    { label: 'Unchanged', value: 9 },
    { label: 'Removed', value: 1 },
  ]);
});
