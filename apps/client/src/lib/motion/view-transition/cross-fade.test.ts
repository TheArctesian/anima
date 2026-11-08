import { expect, it, vi } from 'vitest';
import { crossFadeView } from './cross-fade';

it('5.7.3 theme change, MOT-042: with view transitions the change runs inside one, and the call does not wait for it', () => {
  const steps: string[] = [];
  let update: (() => void) | undefined;
  const view = {
    startViewTransition: vi.fn((callback: () => void) => {
      steps.push('snapshot');
      update = callback;
    }),
  };
  const result: unknown = crossFadeView(() => steps.push('change'), view);
  expect(result).toBeUndefined();
  expect(view.startViewTransition).toHaveBeenCalledOnce();
  // The browser takes the old snapshot first and runs the change on its next frame.
  expect(steps).toEqual(['snapshot']);
  update?.();
  expect(steps).toEqual(['snapshot', 'change']);
});

it('MV-006: without view transitions the change runs at once, with no fade', () => {
  const change = vi.fn();
  crossFadeView(change, {});
  expect(change).toHaveBeenCalledOnce();
});
