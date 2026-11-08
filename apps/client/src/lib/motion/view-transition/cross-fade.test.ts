import { afterEach, expect, it, vi } from 'vitest';
import { crossFadeView } from './cross-fade';

it('5.7.3 theme change, MOT-042: with view transitions the change runs inside one, and the call does not wait for it', () => {
  const steps: string[] = [];
  let update: (() => void) | undefined;
  const view = {
    startViewTransition: vi.fn((callback: () => void) => {
      steps.push('snapshot');
      update = callback;
      return undefined;
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

const unhandled: unknown[] = [];
const record = (reason: unknown) => unhandled.push(reason);

afterEach(() => {
  process.off('unhandledRejection', record);
  unhandled.length = 0;
});

it('MV-006: a transition the browser skips raises no unhandled rejection, and the change still applies', async () => {
  process.on('unhandledRejection', record);
  const skipped = () => Promise.reject(new DOMException('Transition was skipped', 'AbortError'));
  const change = vi.fn();
  crossFadeView(change, {
    startViewTransition: (update) => {
      update();
      return { ready: skipped(), finished: skipped(), updateCallbackDone: skipped() };
    },
  });
  await new Promise((resolve) => setTimeout(resolve, 0));
  expect(change).toHaveBeenCalledOnce();
  expect(unhandled).toEqual([]);
});
