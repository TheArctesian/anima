// @vitest-environment jsdom
import { describe, expect, it } from 'vitest';
import { duration } from '$lib/motion/tokens';
import { grow } from './grow';

const moving = { reduced: false };
const reduced = { reduced: true };

function box(left: number, top: number, width: number, height: number): DOMRect {
  return new DOMRect(left, top, width, height);
}

describe('grow (Session tab open, 5.7.3)', () => {
  const tab = () => {
    const node = document.createElement('div');
    node.getBoundingClientRect = () => box(100, 0, 100, 36);
    return node;
  };
  const origin = { left: 400, top: 300, width: 50, height: 20 };

  /** Where `css` puts the tab's box at its scale about its transform-origin. */
  function drawn(css: string, tab: DOMRect): DOMRect {
    const scale = Number(/scale: ([\d.]+);/.exec(css)?.[1]);
    const [x = 0, y = 0] = (/transform-origin: ([-\d.]+)px ([-\d.]+)px;/.exec(css) ?? []).slice(1).map(Number);
    const fixedX = tab.left + x;
    const fixedY = tab.top + y;
    return box(fixedX + scale * (tab.left - fixedX), fixedY + scale * (tab.top - fixedY), scale * tab.width, scale * tab.height);
  }

  it('MOT-005: the tab starts centred on the control that opened it, at its width, and grows on spring-snappy', () => {
    const config = grow(tab(), { origin, preference: moving })({ direction: 'in' });
    const start = drawn(config.css?.(0, 1) ?? '', box(100, 0, 100, 36));
    expect(start.left).toBeCloseTo(origin.left, 6);
    expect(start.width).toBeCloseTo(origin.width, 6);
    expect(start.top + start.height / 2).toBeCloseTo(origin.top + origin.height / 2, 6);
    expect(config.css?.(1, 0)).toContain('scale: 1;');
    expect(config.duration).toBeGreaterThan(0);
    expect(config.easing?.(1)).toBeCloseTo(1);
  });

  it('MOT-002: a close mid-open goes back from where the tab is, with no step in scale or place', () => {
    const role = grow(tab(), { origin, preference: moving });
    const opening = role({ direction: 'in' });
    const closing = role({ direction: 'out' });
    for (const t of [0.1, 0.4, 0.9]) expect(closing.css?.(t, 1 - t)).toBe(opening.css?.(t, 1 - t));
    expect(closing.duration).toBe(duration.fast);
  });

  it('MOT-002: a reopen mid-close grows back from where the closing tab is', () => {
    const role = grow(tab(), { origin, preference: moving });
    const closing = role({ direction: 'out' });
    const reopening = role({ direction: 'in' });
    for (const t of [0.2, 0.7]) expect(reopening.css?.(t, 1 - t)).toBe(closing.css?.(t, 1 - t));
  });

  it('MOT-040: under reduced motion the tab fades in place over duration-base', () => {
    const config = grow(tab(), { origin, preference: reduced })({ direction: 'in' });
    expect(config.css?.(0.5, 0.5)).toBe('opacity: 0.5;');
    expect(config.duration).toBe(duration.base);
  });

  it('a tab with no known control fades in', () => {
    const config = grow(tab(), { origin: null, preference: moving })({ direction: 'in' });
    expect(config.css?.(0.5, 0.5)).toBe('opacity: 0.5;');
  });

  it('5.7.3: a closing tab closes to its own place over duration-fast; reduced motion only fades', () => {
    const exit = grow(tab(), { origin, preference: moving })({ direction: 'out' });
    expect(exit.css?.(0.5, 0.5)).toBe('opacity: 0.5; scale: 0.5;');
    expect(exit.css?.(0.5, 0.5)).not.toContain('transform-origin');
    expect(exit.duration).toBe(duration.fast);
    expect(grow(tab(), { origin, preference: reduced })({ direction: 'out' }).css?.(0.5, 0.5)).toBe('opacity: 0.5;');
  });
});
