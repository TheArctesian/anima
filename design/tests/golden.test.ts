import { beforeAll, describe, expect, test } from 'bun:test';
import { resolve } from 'node:path';
import { buildOutputs, OUTPUT_PATHS } from '../src/outputs.ts';
import { sampleCurve, settleMs, stepResponse } from '../src/spring.ts';

const root = resolve(import.meta.dir, '..', '..');
let css = '';

beforeAll(async () => {
  css = (await buildOutputs(root))[OUTPUT_PATHS.css] ?? '';
});

/** Independent check: integrate x'' = -k(x-1) - c x' with RK4 from rest at 0. */
function integrate(dampingRatio: number, responseS: number, seconds: number): number {
  const k = (2 * Math.PI / responseS) ** 2;
  const c = (4 * Math.PI * dampingRatio) / responseS;
  const accel = (x: number, v: number): number => -k * (x - 1) - c * v;
  const steps = 100_000;
  const h = seconds / steps;
  let x = 0;
  let v = 0;
  for (let i = 0; i < steps; i += 1) {
    const k1x = v, k1v = accel(x, v);
    const k2x = v + (h / 2) * k1v, k2v = accel(x + (h / 2) * k1x, v + (h / 2) * k1v);
    const k3x = v + (h / 2) * k2v, k3v = accel(x + (h / 2) * k2x, v + (h / 2) * k2v);
    const k4x = v + h * k3v, k4v = accel(x + h * k3x, v + h * k3v);
    x += (h / 6) * (k1x + 2 * k2x + 2 * k3x + k4x);
    v += (h / 6) * (k1v + 2 * k2v + 2 * k3v + k4v);
  }
  return x;
}

describe('step response golden values (5.7.2)', () => {
  test('5.7.2 every spring starts at 0', () => {
    expect(stepResponse({ dampingRatio: 1, responseS: 0.35 }, 0)).toBe(0);
    expect(stepResponse({ dampingRatio: 0.8, responseS: 0.3 }, 0)).toBe(0);
  });

  test('5.7.2 critically damped spring at t = response equals 1 - e^(-2π)(1 + 2π)', () => {
    const expected = 1 - Math.exp(-2 * Math.PI) * (1 + 2 * Math.PI);
    expect(stepResponse({ dampingRatio: 1, responseS: 0.35 }, 0.35)).toBeCloseTo(expected, 12);
  });

  test('5.7.2 underdamped spring (0.8, 0.3 s) at t = response matches an RK4 integration', () => {
    const spring = { dampingRatio: 0.8, responseS: 0.3 };
    expect(stepResponse(spring, 0.3)).toBeCloseTo(integrate(0.8, 0.3, 0.3), 6);
    expect(stepResponse(spring, 0.17)).toBeCloseTo(integrate(0.8, 0.3, 0.17), 6);
  });
});

describe('CSS spring curves (D-4)', () => {
  const springs = {
    default: { dampingRatio: 1, responseS: 0.35 },
    snappy: { dampingRatio: 1, responseS: 0.25 },
    move: { dampingRatio: 1, responseS: 0.4 },
    momentum: { dampingRatio: 0.8, responseS: 0.3 },
  };
  for (const [name, spring] of Object.entries(springs)) {
    test(`5.7.2 --spring-${name} carries the settle time and the sampled curve`, () => {
      const match = new RegExp(`--spring-${name}: (\\d+)ms linear\\(([^)]*)\\);`).exec(css);
      expect(Number(match?.[1])).toBe(settleMs(spring));
      expect((match?.[2] ?? '').split(', ').map(Number)).toEqual(sampleCurve(spring));
    });
  }
});

const SPEC_VALUES: Record<string, string> = {
  'radius-sm': '4px', 'radius-md': '6px', 'radius-lg': '10px', 'radius-pill': '999px',
  'border-thin': '1px', 'border-focus': '2px',
  'icon-sm': '1rem', 'icon-md': '1.125rem', 'icon-lg': '1.25rem',
  'control-sm': '1.75rem', 'control-md': '2rem', 'control-lg': '2.5rem',
  'content-form': '44rem', 'content-page': '90rem',
  'target-min-desktop': '24px', 'target-min-touch': '44px',
  'layer-base': '0', 'layer-sticky': '10', 'layer-nav': '20', 'layer-popover': '30', 'layer-modal': '40', 'layer-toast': '50',
  'weight-regular': '400', 'weight-bold': '700',
  'leading-tight': '1.2', 'leading-normal': '1.5', 'leading-relaxed': '1.7',
  'text-xs': '0.75rem', 'text-sm': '0.8125rem', 'text-md': '0.875rem', 'text-lg': '1rem', 'text-xl': '1.25rem', 'text-2xl': '1.5rem',
  'tracking-display': '-0.01em', 'tracking-small': '0.01em',
  'space-0': '0', 'space-1': '0.25rem', 'space-2': '0.5rem', 'space-3': '0.75rem', 'space-4': '1rem', 'space-5': '1.25rem',
  'space-6': '1.5rem', 'space-8': '2rem', 'space-10': '2.5rem', 'space-12': '3rem', 'space-16': '4rem',
  'elevation-0': 'none', 'elevation-1': 'none',
  'elevation-2': '0 4px 16px color-mix(in srgb, var(--shadow) 40%, transparent)',
  'elevation-3': '0 12px 40px color-mix(in srgb, var(--shadow) 55%, transparent)',
  'material-chrome-bg': 'color-mix(in srgb, var(--surface-base) 82%, transparent)',
  'material-chrome-blur': '20px', 'material-chrome-saturate': '140%',
  'material-panel-bg': 'color-mix(in srgb, var(--surface-raised) 90%, transparent)',
  'material-panel-blur': '12px', 'material-panel-saturate': '100%',
  'material-overlay-bg': 'color-mix(in srgb, var(--surface-overlay) 92%, transparent)',
  'material-overlay-blur': '24px', 'material-overlay-saturate': '100%',
  'duration-instant': '0ms', 'duration-press': '100ms', 'duration-fast': '150ms', 'duration-base': '200ms', 'duration-highlight': '1500ms',
  'stagger-list': '20ms',
  'ease-enter': 'cubic-bezier(0.2, 0, 0, 1)', 'ease-exit': 'cubic-bezier(1, 0, 0.8, 1)',
  'threshold-drag': '10px', 'threshold-loading': '300ms', 'threshold-loading-min': '500ms',
};

describe('spec values (5.5.3, 5.6, 5.7.2)', () => {
  for (const [name, value] of Object.entries(SPEC_VALUES)) {
    test(`5.5.3 --${name} is ${value}`, () => {
      expect(css).toContain(`  --${name}: ${value};`);
    });
  }
});
