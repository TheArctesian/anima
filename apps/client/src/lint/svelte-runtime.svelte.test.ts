// @vitest-environment jsdom
import { flushSync } from 'svelte';
import { createSubscriber } from 'svelte/reactivity';
import { describe, expect, it } from 'vitest';

// Guards the `browser` resolve condition in vite.config.ts: with the server build of svelte,
// effects never run and these tests fail.
describe('Svelte runtime in tests', () => {
  it('CQ-043: an effect re-runs when a $state value changes', () => {
    const seen: number[] = [];
    let count = $state(0);
    const stop = $effect.root(() => {
      $effect(() => {
        seen.push(count);
      });
    });
    flushSync();
    count = 1;
    flushSync();
    stop();
    expect(seen).toEqual([0, 1]);
  });

  it('CQ-043: createSubscriber starts its source when an effect reads it', () => {
    let started = 0;
    const subscribe = createSubscriber(() => {
      started += 1;
    });
    const stop = $effect.root(() => {
      $effect(() => {
        subscribe();
      });
    });
    flushSync();
    stop();
    expect(started).toBe(1);
  });
});
