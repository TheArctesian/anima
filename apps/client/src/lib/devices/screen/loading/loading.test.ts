import { expect, it } from 'vitest';
import { threshold } from '$lib/motion/tokens';
import { manualClock } from './clock.fixture';
import { skeletonGate } from './loading';

function gate() {
  const time = manualClock();
  const changes: boolean[] = [];
  return { time, changes, gate: skeletonGate(time.clock, (visible) => changes.push(visible)) };
}

it('Loading: data before threshold-loading shows no skeleton', () => {
  const { time, changes, gate: loading } = gate();
  loading.begin();
  time.advance(threshold.loading - 1);
  loading.end();
  time.advance(threshold.loadingMin * 2);
  expect(changes).toEqual([]);
});

it('Loading: the skeleton shows after threshold-loading', () => {
  const { time, changes, gate: loading } = gate();
  loading.begin();
  time.advance(threshold.loading);
  expect(changes).toEqual([true]);
});

it('Loading: once shown it stays threshold-loading-min, even when data comes at once', () => {
  const { time, changes, gate: loading } = gate();
  loading.begin();
  time.advance(threshold.loading);
  time.advance(10);
  loading.end();
  time.advance(threshold.loadingMin - 11);
  expect(changes).toEqual([true]);
  time.advance(1);
  expect(changes).toEqual([true, false]);
});

it('Loading: data after the minimum hides the skeleton at once', () => {
  const { time, changes, gate: loading } = gate();
  loading.begin();
  time.advance(threshold.loading + threshold.loadingMin);
  loading.end();
  expect(changes).toEqual([true, false]);
});

it('Loading: a load that starts during the hold keeps the skeleton and cancels the hide', () => {
  const { time, changes, gate: loading } = gate();
  loading.begin();
  time.advance(threshold.loading);
  loading.end();
  loading.begin();
  time.advance(threshold.loadingMin * 2);
  expect(changes).toEqual([true]);
});

it('Loading: stop hides a shown skeleton and cancels a waiting one', () => {
  const { time, changes, gate: loading } = gate();
  loading.begin();
  loading.stop();
  time.advance(threshold.loading * 2);
  expect(changes).toEqual([]);
  loading.begin();
  time.advance(threshold.loading);
  loading.stop();
  expect(changes).toEqual([true, false]);
});
