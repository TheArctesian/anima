// @vitest-environment jsdom
import { describe, expect, it } from 'vitest';
import { duration, reducedMotion } from '../tokens';
import { crossFade, modalIn, modalOut, popoverIn, popoverOut } from './transitions';

const normal = { preference: { reduced: false } };
const reduced = { preference: { reduced: true } };

function placed(left: number, top: number, width: number, height: number): Element {
  const node = document.createElement('div');
  node.getBoundingClientRect = () => new DOMRect(left, top, width, height);
  return node;
}

function css(config: { css?: (t: number, u: number) => string }, t: number): string {
  return config.css?.(t, 1 - t) ?? '';
}

describe('role transitions', () => {
  it('menu and popover: enter scales from 0.96 to 1 with a fade', () => {
    const config = popoverIn(placed(0, 0, 100, 100), normal);
    expect(css(config, 0)).toContain('opacity: 0;');
    expect(css(config, 0)).toContain('scale: 0.96;');
    expect(css(config, 1)).toContain('scale: 1;');
    expect(config.easing?.(1)).toBeCloseTo(1, 2);
  });

  it('MOT-005: a popover grows from the centre of the control that opened it', () => {
    const control = placed(10, 0, 40, 20);
    const config = popoverIn(placed(0, 30, 200, 100), { ...normal, from: control });
    expect(css(config, 0)).toContain('transform-origin: 30px -20px;');
  });

  it('menu and popover: exit fades and scales to 0.98 over duration-fast', () => {
    const config = popoverOut(placed(0, 0, 100, 100), normal);
    expect(config.duration).toBe(duration.fast);
    expect(css(config, 0)).toContain('scale: 0.98;');
  });

  it('modal: the surface enters from 0.98 and leaves the same way over duration-fast', () => {
    expect(css(modalIn(placed(0, 0, 1, 1), normal), 0)).toContain('scale: 0.98;');
    const out = modalOut(placed(0, 0, 1, 1), normal);
    expect(out.duration).toBe(duration.fast);
    expect(css(out, 0)).toContain('scale: 0.98;');
  });

  it.each([
    ['popoverIn', popoverIn],
    ['popoverOut', popoverOut],
    ['modalIn', modalIn],
    ['modalOut', modalOut],
  ] as const)('MOT-040/041: under reduced motion %s is a fade only', (_name, transition) => {
    const config = transition(placed(0, 0, 1, 1), reduced);
    expect(css(config, 0.5)).toBe('opacity: 0.5;');
  });

  it('page change: a cross-fade over duration-base, the same under reduced motion', () => {
    const node = placed(0, 0, 1, 1);
    expect(crossFade(node, normal).duration).toBe(duration.base);
    expect(crossFade(node, reduced).duration).toBe(reducedMotion.duration.base);
    expect(css(crossFade(node, reduced), 0.25)).toBe('opacity: 0.25;');
  });
});
