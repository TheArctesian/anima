// @vitest-environment jsdom
import { afterEach, expect, it, vi } from 'vitest';
import { resolve } from './resolve';

interface Played {
  readonly frames: Keyframe[];
  readonly delay: number;
}

afterEach(() => {
  document.body.replaceChildren();
  Reflect.deleteProperty(Element.prototype, 'animate');
});

function play(reduced: boolean): { played: Played[]; cancel: ReturnType<typeof vi.fn>; stop: () => void } {
  const played: Played[] = [];
  const cancel = vi.fn();
  Object.defineProperty(Element.prototype, 'animate', {
    configurable: true,
    value: (frames: Keyframe[], options: KeyframeAnimationOptions) => {
      played.push({ frames, delay: Number(options.delay) });
      return { cancel };
    },
  });
  const node = document.createElement('div');
  node.innerHTML = '<h1>Title</h1><p>Line</p><ul></ul>';
  document.body.append(node);
  const stop = resolve({ reduced })(node);
  return { played, cancel, stop };
}

it('MOT-053: each child rises out of a blur, in order, once', () => {
  const { played } = play(false);
  expect(played).toHaveLength(3);
  expect(played[0]?.frames[0]).toMatchObject({ opacity: 0, translate: '0 4px', filter: 'blur(6px)' });
  expect(played.map((one) => one.delay)).toEqual([0, 20, 40]);
});

it('MOT-041, MOT-053: under reduced motion it is a fade, with no rise and no blur', () => {
  const { played } = play(true);
  for (const one of played) {
    for (const frame of one.frames) {
      expect(frame).not.toHaveProperty('filter');
      expect(frame).not.toHaveProperty('translate');
    }
  }
});

it('MOT-053: leaving the page stops what is still playing', () => {
  const { cancel, stop } = play(false);
  stop();
  expect(cancel).toHaveBeenCalledTimes(3);
});
