// @vitest-environment jsdom
import { flushSync } from 'svelte';
import { expect, it, vi } from 'vitest';
import { runs } from '../engine.fixture';
import { sharedItem } from './shared';

vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('../engine.fixture')).fakeRun,
}));

const key = 'device-7-name';

function box(top: number): HTMLElement {
  const node = document.createElement('span');
  node.getBoundingClientRect = () => new DOMRect(0, top, 100, 20);
  document.body.append(node);
  return node;
}

it('card to detail, MOT-041: a change of preference after the attach does not start a second flight', () => {
  box(300).dataset.shared = key;
  const node = box(20);
  let reduced = $state(false);
  const preference = {
    get reduced() {
      return reduced;
    },
  };
  runs.length = 0;

  // Svelte runs an attachment inside an effect, as here.
  const stop = $effect.root(() => {
    $effect(() => sharedItem(key, { preference })(node) ?? undefined);
  });
  flushSync();
  const firstFlight = runs.length;
  reduced = true;
  flushSync();
  reduced = false;
  flushSync();
  stop();

  expect(firstFlight).toBeGreaterThan(0);
  expect(runs).toHaveLength(firstFlight);
});
